use alloc::string::ToString;

use miden_objects::{
    crypto::rand::FeltRng,
    notes::{Note, NoteDetails, NoteFile, NoteId, NoteInclusionProof, NoteMetadata, NoteTag},
};
use winter_maybe_async::maybe_await;

use crate::{
    store::{ExpectedNoteState, InputNoteRecord},
    Client, ClientError,
};

impl<R: FeltRng> Client<R> {
    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store. The information stored depends on the
    /// type of note file provided. If the note existed previously, it will be updated with the
    /// new information.
    ///
    /// - If the note file is a [NoteFile::NoteId], the note is fetched from the node and stored in
    ///   the client's store. If the note is private or does not exist, an error is returned.
    /// - If the note file is a [NoteFile::NoteDetails], a new note is created with the provided
    ///   details and tag.
    /// - If the note file is a [NoteFile::NoteWithProof], the note is stored with the provided
    ///   inclusion proof and metadata. The block header data is only fetched from the node if the
    ///   note is committed in the past relative to the client.
    pub async fn import_note(&mut self, note_file: NoteFile) -> Result<NoteId, ClientError> {
        let id = match &note_file {
            NoteFile::NoteId(id) => *id,
            NoteFile::NoteDetails { details, .. } => details.id(),
            NoteFile::NoteWithProof(note, _) => note.id(),
        };

        let previous_note = maybe_await!(self.get_input_note(id)).ok();

        let note = match note_file {
            NoteFile::NoteId(id) => self.import_note_record_by_id(previous_note, id).await?,
            NoteFile::NoteDetails { details, after_block_num, tag } => {
                self.import_note_record_by_details(previous_note, details, after_block_num, tag)
                    .await?
            },
            NoteFile::NoteWithProof(note, inclusion_proof) => {
                self.import_note_record_by_proof(previous_note, note, inclusion_proof).await?
            },
        };

        if let Some(note) = note {
            maybe_await!(self.store.upsert_input_note(note))?;
        }

        Ok(id)
    }

    // HELPERS
    // ================================================================================================

    /// Builds a note record from the note ID. If a note with the same ID was already stored it is
    /// passed via `previous_note` so it can be updated. The note information is fetched from the
    /// node and stored in the client's store.
    ///
    /// Errors:
    /// - If the note does not exist on the node.
    /// - If the note exists but is private.
    async fn import_note_record_by_id(
        &mut self,
        previous_note: Option<InputNoteRecord>,
        id: NoteId,
    ) -> Result<Option<InputNoteRecord>, ClientError> {
        let mut chain_notes = self.rpc_api.get_notes_by_id(&[id]).await?;
        if chain_notes.is_empty() {
            return Err(ClientError::NoteNotFoundOnChain(id));
        }

        let note_details: crate::rpc::NoteDetails =
            chain_notes.pop().expect("chain_notes should have at least one element");

        let inclusion_details = note_details.inclusion_details();

        // Add the inclusion proof to the imported note
        let inclusion_proof = NoteInclusionProof::new(
            inclusion_details.block_num,
            inclusion_details.note_index,
            inclusion_details.merkle_path.clone(),
        )?;

        match previous_note {
            Some(mut previous_note) => {
                if previous_note
                    .inclusion_proof_received(inclusion_proof, *note_details.metadata())?
                {
                    Ok(Some(previous_note))
                } else {
                    Ok(None)
                }
            },
            None => {
                let node_note = match note_details {
                    crate::rpc::NoteDetails::Public(note, _) => note,
                    crate::rpc::NoteDetails::Private(..) => {
                        return Err(ClientError::NoteImportError(
                            "Incomplete imported note is private".to_string(),
                        ))
                    },
                };

                self.import_note_record_by_proof(previous_note, node_note, inclusion_proof)
                    .await
            },
        }
    }

    /// Builds a note record from the note and inclusion proof. If a note with the same id was
    /// already stored it is passed via `previous_note` so it can be updated. The note's
    /// nullifier is used to determine if the note has been consumed in the node and gives it
    /// the correct status.
    ///
    /// If the note is not consumed and it was committed in the past relative to the client, then
    /// the MMR for the relevant block is fetched from the node and stored.
    async fn import_note_record_by_proof(
        &mut self,
        previous_note: Option<InputNoteRecord>,
        note: Note,
        inclusion_proof: NoteInclusionProof,
    ) -> Result<Option<InputNoteRecord>, ClientError> {
        let metadata = *note.metadata();
        let mut note_record = previous_note.unwrap_or(InputNoteRecord::new(
            note.into(),
            None,
            ExpectedNoteState {
                metadata: Some(metadata),
                after_block_num: inclusion_proof.location().block_num(),
                tag: Some(metadata.tag()),
            }
            .into(),
        ));

        if let Some(block_height) =
            self.rpc_api.get_nullifier_commit_height(&note_record.nullifier()).await?
        {
            if note_record.nullifier_received(note_record.nullifier(), block_height)? {
                return Ok(Some(note_record));
            }

            Ok(None)
        } else {
            let block_height = inclusion_proof.location().block_num();
            let current_block_num = maybe_await!(self.get_sync_height())?;

            let mut note_changed =
                note_record.inclusion_proof_received(inclusion_proof, metadata)?;

            if block_height < current_block_num {
                let mut current_partial_mmr = maybe_await!(self.build_current_partial_mmr(true))?;

                let block_header = self
                    .get_and_store_authenticated_block(block_height, &mut current_partial_mmr)
                    .await?;

                note_changed |= note_record.block_header_received(block_header)?;
            }

            if note_changed {
                Ok(Some(note_record))
            } else {
                Ok(None)
            }
        }
    }

    /// Builds a note record from the note details. If a note with the same id was already stored it
    /// is passed via `previous_note` so it can be updated.
    async fn import_note_record_by_details(
        &mut self,
        previous_note: Option<InputNoteRecord>,
        details: NoteDetails,
        after_block_num: u32,
        tag: Option<NoteTag>,
    ) -> Result<Option<InputNoteRecord>, ClientError> {
        let mut note_record = previous_note.unwrap_or({
            InputNoteRecord::new(
                details,
                None,
                ExpectedNoteState { metadata: None, after_block_num, tag }.into(),
            )
        });

        let committed_note_data = if let Some(tag) = tag {
            self.check_expected_note(after_block_num, tag, note_record.details()).await?
        } else {
            None
        };

        match committed_note_data {
            Some((metadata, inclusion_proof)) => {
                let mut current_partial_mmr = maybe_await!(self.build_current_partial_mmr(true))?;
                let block_header = self
                    .get_and_store_authenticated_block(
                        inclusion_proof.location().block_num(),
                        &mut current_partial_mmr,
                    )
                    .await?;

                let note_changed =
                    note_record.inclusion_proof_received(inclusion_proof, metadata)?;

                if note_record.block_header_received(block_header)? | note_changed {
                    Ok(Some(note_record))
                } else {
                    Ok(None)
                }
            },
            None => Ok(Some(note_record)),
        }
    }

    /// Checks if a note with the given note_tag and id is present in the chain between the
    /// `request_block_num` and the current block. If found it returns its metadata and inclusion
    /// proof.
    async fn check_expected_note(
        &mut self,
        mut request_block_num: u32,
        tag: NoteTag,
        expected_note: &miden_objects::notes::NoteDetails,
    ) -> Result<Option<(NoteMetadata, NoteInclusionProof)>, ClientError> {
        let current_block_num = maybe_await!(self.get_sync_height())?;
        loop {
            if request_block_num > current_block_num {
                return Ok(None);
            };

            let sync_notes = self.rpc_api.sync_notes(request_block_num, &[tag]).await?;

            if sync_notes.block_header.block_num() == sync_notes.chain_tip {
                return Ok(None);
            }

            // This means that notes with that note_tag were found.
            // Therefore, we should check if a note with the same id was found.
            let committed_note =
                sync_notes.notes.iter().find(|note| note.note_id() == &expected_note.id());

            if let Some(note) = committed_note {
                // This means that a note with the same id was found.
                // Therefore, we should mark the note as committed.
                let note_block_num = sync_notes.block_header.block_num();

                if note_block_num > current_block_num {
                    return Ok(None);
                };

                let note_inclusion_proof = NoteInclusionProof::new(
                    note_block_num,
                    note.note_index(),
                    note.merkle_path().clone(),
                )?;

                return Ok(Some((note.metadata(), note_inclusion_proof)));
            } else {
                // This means that a note with the same id was not found.
                // Therefore, we should request again for sync_notes with the same note_tag
                // and with the block_num of the last block header
                // (sync_notes.block_header.unwrap()).
                request_block_num = sync_notes.block_header.block_num();
            }
        }
    }
}

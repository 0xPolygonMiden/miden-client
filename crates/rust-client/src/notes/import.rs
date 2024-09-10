use alloc::string::ToString;

use miden_objects::{
    crypto::rand::FeltRng,
    notes::{
        compute_note_hash, Note, NoteDetails, NoteFile, NoteId, NoteInclusionProof, NoteMetadata,
        NoteTag,
    },
};
use miden_tx::auth::TransactionAuthenticator;
use winter_maybe_async::maybe_await;

use crate::{
    rpc::NodeRpcClient,
    store::{InputNoteRecord, NoteState, Store, StoreError},
    Client, ClientError,
};

impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> Client<N, R, S, A> {
    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store. The information stored depends on the
    /// type of note file provided.
    ///
    /// - If the note file is a [NoteFile::NoteId], the note is fetched from the node and stored in
    ///   the client's store. If the note is private or does not exist, an error is returned. If the
    ///   ID was already stored, the inclusion proof and metadata are updated.
    /// - If the note file is a [NoteFile::NoteDetails], a new note is created with the provided
    ///   details. The note is marked as ignored if it contains no tag or if the tag is not
    ///   relevant.
    /// - If the note file is a [NoteFile::NoteWithProof], the note is stored with the provided
    ///   inclusion proof and metadata. The MMR data is only fetched from the node if the note is
    ///   committed in the past relative to the client.
    pub async fn import_note(&mut self, note_file: NoteFile) -> Result<NoteId, ClientError> {
        let note = match note_file {
            NoteFile::NoteId(id) => self.import_note_record_by_id(id).await?,
            NoteFile::NoteDetails { details, after_block_num, tag } => {
                self.import_note_record_by_details(details, after_block_num, tag).await?
            },
            NoteFile::NoteWithProof(note, inclusion_proof) => {
                self.import_note_record_by_proof(note, inclusion_proof).await?
            },
        };
        let id = note.id();

        if maybe_await!(self.get_input_note(id)).is_ok() {
            return Err(ClientError::NoteImportError(format!(
                "Note with ID {} already exists",
                id
            )));
        }

        maybe_await!(self.store.insert_input_note(note))?;
        Ok(id)
    }

    // HELPERS
    // ================================================================================================

    /// Builds a note record from the note id. The note information is fetched from the node and
    /// stored in the client's store. If the note already exists in the store, the inclusion proof
    /// and metadata are updated.
    ///
    /// Errors:
    /// - If the note does not exist on the node.
    /// - If the note exists but is private.
    async fn import_note_record_by_id(
        &mut self,
        id: NoteId,
    ) -> Result<InputNoteRecord, ClientError> {
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

        let store_note = maybe_await!(self.get_input_note(id));

        match store_note {
            Ok(mut store_note) => {
                store_note.inclusion_proof_received(inclusion_proof, *note_details.metadata())?;
                Ok(store_note)
            },
            Err(ClientError::StoreError(StoreError::NoteNotFound(_))) => {
                let node_note = match note_details {
                    crate::rpc::NoteDetails::Public(note, _) => note,
                    crate::rpc::NoteDetails::Private(..) => {
                        return Err(ClientError::NoteImportError(
                            "Incomplete imported note is private".to_string(),
                        ))
                    },
                };

                self.import_note_record_by_proof(node_note, inclusion_proof).await
            },
            Err(err) => Err(err),
        }
    }

    /// Builds a note record from the note and inclusion proof. The note's nullifier is used to
    /// determine if the note has been consumed in the node and gives it the correct status.
    ///
    /// If the note is not consumed and it was committed in the past relative to the client, then
    /// the MMR for the relevant block is fetched from the node and stored.
    async fn import_note_record_by_proof(
        &mut self,
        note: Note,
        inclusion_proof: NoteInclusionProof,
    ) -> Result<InputNoteRecord, ClientError> {
        let state = if let Some(block_height) =
            self.rpc_api.get_nullifier_commit_height(&note.nullifier()).await?
        {
            NoteState::ForeignConsumed { nullifier_block_height: block_height }
        } else {
            let block_height = inclusion_proof.location().block_num();
            let current_block_num = maybe_await!(self.get_sync_height())?;

            if block_height < current_block_num {
                let mut current_partial_mmr = maybe_await!(self.build_current_partial_mmr(true))?;

                self.get_and_store_authenticated_block(block_height, &mut current_partial_mmr)
                    .await?;
            }
            NoteState::Unverified {
                metadata: *note.metadata(),
                inclusion_proof,
            }
        };

        Ok(InputNoteRecord::new(note.into(), None, state))
    }

    /// Builds a note record from the note details. If a tag is not provided or not tracked, the
    /// note is marked as ignored.
    async fn import_note_record_by_details(
        &mut self,
        details: NoteDetails,
        after_block_num: u32,
        tag: Option<NoteTag>,
    ) -> Result<InputNoteRecord, ClientError> {
        match tag {
            Some(tag) => {
                let commited_note_data =
                    self.check_expected_note(after_block_num, tag, &details).await?;

                match commited_note_data {
                    Some((metadata, inclusion_proof)) => {
                        let mut current_partial_mmr =
                            maybe_await!(self.build_current_partial_mmr(true))?;
                        let block_header = self
                            .get_and_store_authenticated_block(
                                inclusion_proof.location().block_num(),
                                &mut current_partial_mmr,
                            )
                            .await?;

                        let state = if inclusion_proof.note_path().verify(
                            inclusion_proof.location().node_index_in_block().into(),
                            compute_note_hash(details.id(), &metadata),
                            &block_header.note_root(),
                        ) {
                            NoteState::Invalid {
                                metadata,
                                invalid_inclusion_proof: inclusion_proof,
                                block_note_root: block_header.note_root(),
                            }
                        } else {
                            NoteState::Committed {
                                metadata,
                                inclusion_proof,
                                block_note_root: block_header.note_root(),
                            }
                        };

                        Ok(InputNoteRecord::new(details, None, state))
                    },
                    None => Ok(InputNoteRecord::new(
                        details,
                        None,
                        NoteState::Expected { after_block_num, tag },
                    )),
                }
            },
            None => Ok(InputNoteRecord::new(details, None, NoteState::Unknown)),
        }
    }

    /// Synchronizes a note with the chain.
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

use alloc::{string::ToString, vec::Vec};

use miden_objects::{
    crypto::rand::FeltRng,
    notes::{
        Note, NoteDetails, NoteExecutionMode, NoteFile, NoteId, NoteInclusionProof, NoteTag,
        Nullifier,
    },
    transaction::InputNote,
};
use miden_tx::auth::TransactionAuthenticator;
use tracing::info;
use winter_maybe_async::maybe_await;

use crate::{
    rpc::NodeRpcClient,
    store::{InputNoteRecord, NoteFilter, NoteStatus, Store, StoreError},
    sync::get_nullifier_prefix,
    Client, ClientError,
};

impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> Client<N, R, S, A> {
    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store. The information stored depends on the
    /// type of note file provided.
    ///
    /// If the note file is a [NoteFile::NoteId], the note is fecthed from the node and stored in
    /// the client's store. If the note is private or does not exist, an error is returned. If the
    /// ID was already stored, the inclusion proof and metadata are updated.
    /// If the note file is a [NoteFile::NoteDetails], a new note is created with the provided
    /// details. The note is marked as ignored if it contains no tag or if the tag is not relevant.
    /// If the note file is a [NoteFile::NoteWithProof], the note is stored with the provided
    /// inclusion proof and metadata. The MMR data is not fetched from the node.
    pub async fn import_note(&mut self, note_file: NoteFile) -> Result<NoteId, ClientError> {
        let note = match note_file {
            NoteFile::NoteId(id) => {
                let note_record = self.get_note_record_by_id(id).await?;
                if note_record.is_none() {
                    return Ok(id);
                }

                note_record.expect("The note record should be Some")
            },
            NoteFile::NoteDetails { details, after_block_num, tag } => {
                self.get_note_record_by_details(details, after_block_num, tag).await?
            },
            NoteFile::NoteWithProof(note, inclusion_proof) => {
                self.get_note_record_by_proof(note, inclusion_proof).await?
            },
        };
        let id = note.id();

        maybe_await!(self.store.insert_input_note(note))?;
        Ok(id)
    }

    // HELPERS
    // ================================================================================================
    async fn get_note_record_by_id(
        &mut self,
        id: NoteId,
    ) -> Result<Option<InputNoteRecord>, ClientError> {
        let mut chain_notes = self.rpc_api.get_notes_by_id(&[id]).await?;
        if chain_notes.is_empty() {
            return Err(ClientError::ExistenceVerificationError(id));
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

        let tracked_note = maybe_await!(self.get_input_note(id));

        if let Err(ClientError::StoreError(StoreError::NoteNotFound(_))) = tracked_note {
            let node_note = match note_details {
                crate::rpc::NoteDetails::Public(note, _) => note,
                crate::rpc::NoteDetails::OffChain(..) => {
                    return Err(ClientError::NoteImportError(
                        "Incomplete imported note is private".to_string(),
                    ))
                },
            };

            // If note is not tracked, we create a new one.
            let details = node_note.clone().into();

            let status = if let Some(block_height) =
                self.nullifier_block_num(&node_note.nullifier()).await?
            {
                NoteStatus::Consumed { consumer_account_id: None, block_height }
            } else {
                NoteStatus::Committed {
                    block_height: inclusion_proof.location().block_num() as u64,
                }
            };

            Ok(Some(InputNoteRecord::new(
                node_note.id(),
                node_note.recipient().digest(),
                node_note.assets().clone(),
                status,
                Some(*node_note.metadata()),
                Some(inclusion_proof),
                details,
                false,
                None,
            )))
        } else {
            // If note is already tracked, we update the inclusion proof and metadata.
            let tracked_note = tracked_note?;

            // TODO: Join these calls to one method that updates both fields with one query (issue #404)
            maybe_await!(self
                .store
                .update_note_inclusion_proof(tracked_note.id(), inclusion_proof))?;
            maybe_await!(self
                .store
                .update_note_metadata(tracked_note.id(), *note_details.metadata()))?;

            Ok(None)
        }
    }

    async fn get_note_record_by_proof(
        &mut self,
        note: Note,
        inclusion_proof: NoteInclusionProof,
    ) -> Result<InputNoteRecord, ClientError> {
        let details = note.clone().into();

        let status = if let Some(block_height) = self.nullifier_block_num(&note.nullifier()).await?
        {
            NoteStatus::Consumed { consumer_account_id: None, block_height }
        } else {
            NoteStatus::Committed {
                block_height: inclusion_proof.location().block_num() as u64,
            }
        };

        Ok(InputNoteRecord::new(
            note.id(),
            note.recipient().digest(),
            note.assets().clone(),
            status,
            Some(*note.metadata()),
            Some(inclusion_proof),
            details,
            false,
            None,
        ))
    }

    async fn get_note_record_by_details(
        &mut self,
        details: NoteDetails,
        after_block_num: u32,
        tag: Option<NoteTag>,
    ) -> Result<InputNoteRecord, ClientError> {
        let record_details = details.clone().into();

        match tag {
            Some(tag) => {
                let tracked_tags = maybe_await!(self.get_note_tags())?;

                let account_tags = maybe_await!(self.get_account_stubs())?
                    .into_iter()
                    .map(|(stub, _)| NoteTag::from_account_id(stub.id(), NoteExecutionMode::Local))
                    .collect::<Result<Vec<_>, _>>()?;

                let uncommited_note_tags =
                    maybe_await!(self.get_input_notes(NoteFilter::Expected))?
                        .into_iter()
                        .filter_map(|note| note.metadata().map(|metadata| metadata.tag()))
                        .collect::<Vec<_>>();

                let ignored =
                    ![tracked_tags, account_tags, uncommited_note_tags].concat().contains(&tag);

                if ignored {
                    info!("Ignoring note with tag {}", tag);
                }

                if let (NoteStatus::Committed { block_height }, Some(input_note)) =
                    self.check_expected_note(after_block_num, tag, &details).await?
                {
                    let mut current_partial_mmr =
                        maybe_await!(self.build_current_partial_mmr(true))?;
                    self.get_and_store_authenticated_block(
                        block_height.try_into().map_err(|_| {
                            ClientError::NoteImportError(
                                "Couldn't convert block height".to_string(),
                            )
                        })?,
                        &mut current_partial_mmr,
                    )
                    .await?;
                    Ok(InputNoteRecord::from(input_note))
                } else {
                    Ok(InputNoteRecord::new(
                        details.id(),
                        details.recipient().digest(),
                        details.assets().clone(),
                        NoteStatus::Expected { created_at: 0 },
                        None,
                        None,
                        record_details,
                        ignored,
                        Some(tag),
                    ))
                }
            },
            None => Ok(InputNoteRecord::new(
                details.id(),
                details.recipient().digest(),
                details.assets().clone(),
                NoteStatus::Expected { created_at: 0 },
                None,
                None,
                record_details,
                true,
                None,
            )),
        }
    }

    async fn nullifier_block_num(
        &mut self,
        nullifier: &Nullifier,
    ) -> Result<Option<u64>, ClientError> {
        let nullifiers = self
            .rpc_api
            .check_nullifiers_by_prefix(&[get_nullifier_prefix(nullifier)])
            .await
            .map_err(ClientError::RpcError)?;

        Ok(nullifiers
            .iter()
            .find(|(n, _)| n == nullifier)
            .map(|(_, block_num)| *block_num as u64))
    }

    /// Synchronizes a note with the chain.
    async fn check_expected_note(
        &mut self,
        mut request_block_num: u32,
        tag: NoteTag,
        expected_note: &miden_objects::notes::NoteDetails,
    ) -> Result<(NoteStatus, Option<InputNote>), ClientError> {
        let current_block_num = maybe_await!(self.get_sync_height())?;
        loop {
            if request_block_num > current_block_num {
                return Ok((NoteStatus::Expected { created_at: 0 }, None));
            };

            let sync_notes = self.rpc_api().sync_notes(request_block_num, &[tag]).await?;

            if sync_notes.block_header.block_num() == sync_notes.chain_tip {
                return Ok((NoteStatus::Expected { created_at: 0 }, None));
            }

            // This means that notes with that note_tag were found.
            // Therefore, we should check if a note with the same id was found.
            let committed_note =
                sync_notes.notes.iter().find(|note| note.note_id() == &expected_note.id());

            if let Some(note) = committed_note {
                // This means that a note with the same id was found.
                // Therefore, we should mark the note as committed.
                let note_block_num = sync_notes.block_header.block_num();

                let note_inclusion_proof = NoteInclusionProof::new(
                    note_block_num,
                    note.note_index(),
                    note.merkle_path().clone(),
                )?;

                return Ok((
                    NoteStatus::Committed {
                        // Block header can't be None since we check that already in the if statement.
                        block_height: note_block_num as u64,
                    },
                    Some(InputNote::authenticated(
                        Note::new(
                            expected_note.assets().clone(),
                            note.metadata(),
                            expected_note.recipient().clone(),
                        ),
                        note_inclusion_proof,
                    )),
                ));
            } else {
                // This means that a note with the same id was not found.
                // Therefore, we should request again for sync_notes with the same note_tag
                // and with the block_num of the last block header (sync_notes.block_header.unwrap()).
                request_block_num = sync_notes.block_header.block_num();
            }
        }
    }
}

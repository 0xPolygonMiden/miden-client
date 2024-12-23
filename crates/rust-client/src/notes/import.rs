use alloc::{boxed::Box, string::ToString, sync::Arc, vec::Vec};

use miden_objects::{
    accounts::AccountHeader,
    crypto::rand::FeltRng,
    notes::{Note, NoteDetails, NoteFile, NoteId, NoteInclusionProof, NoteTag, Nullifier},
};
use tonic::async_trait;

use super::NoteUpdates;
use crate::{
    components::sync_state::{StateSyncUpdate, SyncState, SyncStatus},
    rpc::{domain::notes::NoteDetails as RpcNoteDetails, NodeRpcClient},
    store::{input_note_states::ExpectedNoteState, InputNoteRecord, InputNoteState},
    sync::NoteTagRecord,
    Client, ClientError,
};

/// Note importing methods.
impl<R: FeltRng> Client<R> {
    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store. The information stored depends on the
    /// type of note file provided. If the note existed previously, it will be updated with the
    /// new information. The tag specified by the `NoteFile` will start being tracked.
    ///
    /// - If the note file is a [NoteFile::NoteId], the note is fetched from the node and stored in
    ///   the client's store. If the note is private or doesn't exist, an error is returned.
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

        let previous_note = self.get_input_note(id).await.ok();

        // If the note is already in the store and is in the state processing we return an error.
        if let Some(true) = previous_note.as_ref().map(|note| note.is_processing()) {
            return Err(ClientError::NoteImportError(format!(
                "Can't overwrite note with id {} as it's currently being processed",
                id
            )));
        }

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
            if let InputNoteState::Expected(ExpectedNoteState { tag: Some(tag), .. }) = note.state()
            {
                self.store
                    .add_note_tag(NoteTagRecord::with_note_source(*tag, note.id()))
                    .await?;
            }
            self.store.upsert_input_notes(&[note]).await?;
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
    /// - If the note doesn't exist on the node.
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

        let note_details: RpcNoteDetails =
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
                    self.store.remove_note_tag((&previous_note).try_into()?).await?;

                    Ok(Some(previous_note))
                } else {
                    Ok(None)
                }
            },
            None => {
                let node_note = match note_details {
                    RpcNoteDetails::Public(note, _) => note,
                    RpcNoteDetails::Private(..) => {
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

    /// Builds a note record from the note and inclusion proof. If a note with the same ID was
    /// already stored it is passed via `previous_note` so it can be updated. The note's
    /// nullifier is used to determine if the note has been consumed in the node and gives it
    /// the correct state.
    ///
    /// If the note isn't consumed and it was committed in the past relative to the client, then
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
            self.store.get_current_timestamp(),
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
            if note_record.consumed_externally(note_record.nullifier(), block_height)? {
                return Ok(Some(note_record));
            }

            Ok(None)
        } else {
            let block_height = inclusion_proof.location().block_num();
            let current_block_num = self.get_sync_height().await?;

            let mut note_changed =
                note_record.inclusion_proof_received(inclusion_proof, metadata)?;

            if block_height < current_block_num {
                let mut current_partial_mmr = self.store.build_current_partial_mmr(true).await?;

                let block_header = self
                    .get_and_store_authenticated_block(block_height, &mut current_partial_mmr)
                    .await?;

                note_changed |= note_record.block_header_received(block_header)?;
            }

            if note_changed {
                self.store.remove_note_tag((&note_record).try_into()?).await?;

                Ok(Some(note_record))
            } else {
                Ok(None)
            }
        }
    }

    /// Builds a note record from the note details. If a note with the same ID was already stored it
    /// is passed via `previous_note` so it can be updated.
    async fn import_note_record_by_details(
        &mut self,
        previous_note: Option<InputNoteRecord>,
        details: NoteDetails,
        after_block_num: u32,
        tag: Option<NoteTag>,
    ) -> Result<Option<InputNoteRecord>, ClientError> {
        let note_record = previous_note.unwrap_or({
            InputNoteRecord::new(
                details,
                self.store.get_current_timestamp(),
                ExpectedNoteState { metadata: None, after_block_num, tag }.into(),
            )
        });

        if tag.is_none() {
            return Ok(Some(note_record));
        }

        let tag = tag.expect("tag should be Some");
        let mut sync_block_num = after_block_num;
        let mut sync_state = SingleNoteSync::new(note_record.clone(), self.rpc_api.clone());

        loop {
            let response = sync_state.step_sync_state(sync_block_num, vec![], &[tag], &[]).await?;

            match response {
                None => return Ok(Some(note_record)),
                Some(SyncStatus::SyncedToLastBlock(update))
                | Some(SyncStatus::SyncedToBlock(update)) => {
                    if let Some(new_note_record) = update.note_updates.updated_input_notes().first()
                    {
                        return Ok(Some(new_note_record.clone()));
                    } else {
                        sync_block_num = update.block_header.block_num();
                    }
                },
            }
        }
    }
}

struct SingleNoteSync {
    rpc_api: Arc<dyn NodeRpcClient + Send>,
    note_record: InputNoteRecord,
}

impl SingleNoteSync {
    fn new(note_record: InputNoteRecord, rpc_api: Arc<dyn NodeRpcClient + Send>) -> Self {
        Self { note_record, rpc_api }
    }
}

#[async_trait(?Send)]
impl SyncState for SingleNoteSync {
    async fn step_sync_state(
        &mut self,
        current_block_num: u32,
        _tracked_accounts: Vec<AccountHeader>,
        note_tags: &[NoteTag],
        _nullifiers: &[Nullifier],
    ) -> Result<Option<SyncStatus>, ClientError> {
        let tag = note_tags.first().expect("note_tags should have at least one element");
        let sync_notes = self.rpc_api.sync_notes(current_block_num, &[*tag]).await?;

        if sync_notes.block_header.block_num() == current_block_num {
            return Ok(None);
        }

        // This means that notes with that note_tag were found.
        // Therefore, we should check if a note with the same id was found.
        let committed_note =
            sync_notes.notes.iter().find(|note| note.note_id() == &self.note_record.id());

        let mut update = StateSyncUpdate::new_empty(sync_notes.block_header);
        if let Some(note) = committed_note {
            // This means that a note with the same id was found.
            // Therefore, we should update it.
            let note_changed = self.note_record.inclusion_proof_received(
                NoteInclusionProof::new(
                    sync_notes.block_header.block_num(),
                    note.note_index(),
                    note.merkle_path().clone(),
                )?,
                note.metadata(),
            )?;

            if !(self.note_record.block_header_received(sync_notes.block_header)? | note_changed) {
                // If note was found but didn't change, we return None (as there is no state change)
                return Ok(None);
            }

            let single_note_update =
                NoteUpdates::new(vec![], vec![], vec![self.note_record.clone()], vec![]);
            update.note_updates = single_note_update;
        }

        if sync_notes.chain_tip == sync_notes.block_header.block_num() {
            Ok(Some(SyncStatus::SyncedToLastBlock(update)))
        } else {
            Ok(Some(SyncStatus::SyncedToBlock(update)))
        }
    }
}

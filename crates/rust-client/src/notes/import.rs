use alloc::string::ToString;

use miden_objects::{
    crypto::rand::FeltRng,
    notes::{Note, NoteDetails, NoteFile, NoteId, NoteInclusionProof, NoteTag},
};
use miden_tx::auth::TransactionAuthenticator;
use tracing::info;
use winter_maybe_async::maybe_await;

use crate::{
    rpc::NodeRpcClient,
    store::{InputNoteRecord, NoteStatus, Store, StoreError},
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
    ///   details. The note is marked as ignored if it contains no tag or if the tag is not relevant.
    /// - If the note file is a [NoteFile::NoteWithProof], the note is stored with the provided
    ///   inclusion proof and metadata. The MMR data is not fetched from the node.
    pub async fn import_note(&mut self, note_file: NoteFile) -> Result<NoteId, ClientError> {
        let note = match note_file {
            NoteFile::NoteId(id) => {
                let note_record = self.get_note_record_by_id(id).await?;
                if note_record.is_none() {
                    return Ok(id);
                }

                note_record.expect("The note record should be Some")
            },
            NoteFile::NoteDetails(details, tag) => {
                self.get_note_record_by_details(details, tag).await?
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

    /// Builds a note record from the note id. The note information is fetched from the node and
    /// stored in the client's store. If the note already exists in the store, the inclusion proof
    /// and metadata are updated.
    ///
    /// Errors:
    /// - If the note does not exist on the node.
    /// - If the note exists but is private.
    async fn get_note_record_by_id(
        &mut self,
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

        let store_note = maybe_await!(self.get_input_note(id));

        match store_note {
            Ok(store_note) => {
                // TODO: Join these calls to one method that updates both fields with one query (issue #404)
                maybe_await!(self
                    .store
                    .update_note_inclusion_proof(store_note.id(), inclusion_proof))?;
                maybe_await!(self
                    .store
                    .update_note_metadata(store_note.id(), *note_details.metadata()))?;

                Ok(None)
            },
            Err(ClientError::StoreError(StoreError::NoteNotFound(_))) => {
                let node_note = match note_details {
                    crate::rpc::NoteDetails::Public(note, _) => note,
                    crate::rpc::NoteDetails::OffChain(..) => {
                        return Err(ClientError::NoteImportError(
                            "Incomplete imported note is private".to_string(),
                        ))
                    },
                };

                self.get_note_record_by_proof(node_note, inclusion_proof).await.map(Some)
            },
            Err(err) => Err(err),
        }
    }

    /// Builds a note record from the note and inclusion proof. The note's nullifier is used to
    /// determine if the note has been consumed in the node and gives it the correct status.
    async fn get_note_record_by_proof(
        &mut self,
        note: Note,
        inclusion_proof: NoteInclusionProof,
    ) -> Result<InputNoteRecord, ClientError> {
        let details = note.clone().into();

        let status = if let Some(block_height) =
            self.rpc_api().get_nullifier_commit_height(&note.nullifier()).await?
        {
            NoteStatus::Consumed { consumer_account_id: None, block_height }
        } else {
            NoteStatus::Committed {
                block_height: inclusion_proof.location().block_num(),
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

    /// Builds a note record from the note details. If a tag is not provided or not tracked, the
    /// note is marked as ignored.
    async fn get_note_record_by_details(
        &mut self,
        details: NoteDetails,
        tag: Option<NoteTag>,
    ) -> Result<InputNoteRecord, ClientError> {
        let record_details = details.clone().into();

        match tag {
            Some(tag) => {
                let ignored = !maybe_await!(self.get_tracked_note_tags())?.contains(&tag);

                if ignored {
                    info!("Ignoring note with tag {}", tag);
                }

                Ok(InputNoteRecord::new(
                    details.id(),
                    details.recipient().digest(),
                    details.assets().clone(),
                    NoteStatus::Expected { created_at: None },
                    None,
                    None,
                    record_details,
                    ignored,
                    Some(tag),
                ))
            },
            None => Ok(InputNoteRecord::new(
                details.id(),
                details.recipient().digest(),
                details.assets().clone(),
                NoteStatus::Expected { created_at: None },
                None,
                None,
                record_details,
                true,
                None,
            )),
        }
    }
}

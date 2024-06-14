use miden_objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    crypto::rand::FeltRng,
    notes::{NoteFile, NoteId, NoteInclusionProof, NoteScript},
};
use miden_tx::{auth::TransactionAuthenticator, ScriptTarget};
use tracing::info;
use winter_maybe_async::{maybe_async, maybe_await};

use super::{
    note_screener::NoteConsumability,
    rpc::{NodeRpcClient, NoteDetails},
    Client,
};
use crate::{
    client::NoteScreener,
    errors::{ClientError, StoreError},
    store::{InputNoteRecord, NoteFilter, NoteRecordDetails, NoteStatus, OutputNoteRecord, Store},
};

impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> Client<N, R, S, A> {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    #[maybe_async]
    pub fn get_input_notes(
        &self,
        filter: NoteFilter<'_>,
    ) -> Result<Vec<InputNoteRecord>, ClientError> {
        maybe_await!(self.store.get_input_notes(filter)).map_err(|err| err.into())
    }

    /// Returns the input notes and their consumability.
    ///
    /// If account_id is None then all consumable input notes are returned.
    #[maybe_async]
    pub fn get_consumable_notes(
        &self,
        account_id: Option<AccountId>,
    ) -> Result<Vec<(InputNoteRecord, Vec<NoteConsumability>)>, ClientError> {
        let commited_notes = maybe_await!(self.store.get_input_notes(NoteFilter::Committed))?;

        let note_screener = NoteScreener::new(self.store.clone());

        let mut relevant_notes = Vec::new();
        for input_note in commited_notes {
            let mut account_relevance =
                maybe_await!(note_screener.check_relevance(&input_note.clone().try_into()?))?;

            if let Some(account_id) = account_id {
                account_relevance.retain(|(id, _)| *id == account_id);
            }

            if account_relevance.is_empty() {
                continue;
            }

            relevant_notes.push((input_note, account_relevance));
        }

        Ok(relevant_notes)
    }

    /// Returns the consumability of the provided note.
    #[maybe_async]
    pub fn get_note_consumability(
        &self,
        note: InputNoteRecord,
    ) -> Result<Vec<NoteConsumability>, ClientError> {
        let note_screener = NoteScreener::new(self.store.clone());
        maybe_await!(note_screener.check_relevance(&note.clone().try_into()?))
            .map_err(|err| err.into())
    }

    /// Returns the input note with the specified hash.
    #[maybe_async]
    pub fn get_input_note(&self, note_id: NoteId) -> Result<InputNoteRecord, ClientError> {
        Ok(maybe_await!(self.store.get_input_notes(NoteFilter::Unique(note_id)))?
            .pop()
            .expect("The vector always has one element for NoteFilter::Unique"))
    }

    // OUTPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns output notes managed by this client.
    #[maybe_async]
    pub fn get_output_notes(
        &self,
        filter: NoteFilter<'_>,
    ) -> Result<Vec<OutputNoteRecord>, ClientError> {
        maybe_await!(self.store.get_output_notes(filter)).map_err(|err| err.into())
    }

    /// Returns the output note with the specified hash.
    #[maybe_async]
    pub fn get_output_note(&self, note_id: NoteId) -> Result<OutputNoteRecord, ClientError> {
        Ok(maybe_await!(self.store.get_output_notes(NoteFilter::Unique(note_id)))?
            .pop()
            .expect("The vector always has one element for NoteFilter::Unique"))
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store. The `verify` parameter dictates whether or
    /// not the method verifies the existence of the note in the chain.
    ///
    /// If the imported note is verified to be on chain and it doesn't contain an inclusion proof
    /// the method tries to build one.
    /// If the verification fails then a [ClientError::ExistenceVerificationError] is raised.
    pub async fn import_note(&mut self, note_file: NoteFile) -> Result<NoteId, ClientError> {
        let note = match note_file {
            NoteFile::NoteId(id) => {
                let mut chain_notes = self.rpc_api.get_notes_by_id(&[id]).await?;
                if chain_notes.is_empty() {
                    return Err(ClientError::ExistenceVerificationError(id));
                }

                let note_details =
                    chain_notes.pop().expect("chain_notes should have at least one element");

                let (note, inclusion_details) = match note_details {
                    NoteDetails::OffChain(..) => {
                        return Err(ClientError::NoteImportError(
                            "Incomplete imported note is private".to_string(),
                        ))
                    },
                    NoteDetails::Public(note, inclusion_proof) => (note, inclusion_proof),
                };

                // Add the inclusion proof to the imported note
                info!("Requesting MMR data for past block num {}", inclusion_details.block_num);
                let mut current_partial_mmr = self.build_current_partial_mmr(true)?;
                let block_header = self
                    .get_and_store_authenticated_block(
                        inclusion_details.block_num,
                        &mut current_partial_mmr,
                    )
                    .await?;
                let inclusion_proof = NoteInclusionProof::new(
                    inclusion_details.block_num,
                    block_header.sub_hash(),
                    block_header.note_root(),
                    inclusion_details.note_index.into(),
                    inclusion_details.merkle_path.clone(),
                )?;

                let details = NoteRecordDetails::new(
                    note.nullifier().to_string(),
                    note.script().clone(),
                    note.inputs().values().to_vec(),
                    note.serial_num(),
                );

                InputNoteRecord::new(
                    note.id(),
                    note.recipient().digest(),
                    note.assets().clone(),
                    NoteStatus::Committed {
                        block_height: inclusion_proof.origin().block_num as u64,
                    },
                    Some(*note.metadata()),
                    Some(inclusion_proof),
                    details,
                    false,
                    None,
                )
            },
            NoteFile::NoteDetails(details, None) => {
                let record_details = NoteRecordDetails::new(
                    details.nullifier().to_string(),
                    details.script().clone(),
                    details.inputs().values().to_vec(),
                    details.serial_num(),
                );

                InputNoteRecord::new(
                    details.id(),
                    details.recipient().digest(),
                    details.assets().clone(),
                    NoteStatus::Expected { created_at: 0 },
                    None,
                    None,
                    record_details,
                    true,
                    None,
                )
            },
            NoteFile::NoteDetails(details, Some(tag)) => {
                let tracked_tags = self.get_note_tags()?;
                let ignored = tracked_tags.contains(&tag);

                let record_details = NoteRecordDetails::new(
                    details.nullifier().to_string(),
                    details.script().clone(),
                    details.inputs().values().to_vec(),
                    details.serial_num(),
                );

                InputNoteRecord::new(
                    details.id(),
                    details.recipient().digest(),
                    details.assets().clone(),
                    NoteStatus::Expected { created_at: 0 },
                    None,
                    None,
                    record_details,
                    ignored,
                    Some(tag),
                )
            },
            NoteFile::NoteWithProof(note, inclusion_proof) => {
                let details = NoteRecordDetails::new(
                    note.nullifier().to_string(),
                    note.script().clone(),
                    note.inputs().values().to_vec(),
                    note.serial_num(),
                );

                InputNoteRecord::new(
                    note.id(),
                    note.recipient().digest(),
                    note.assets().clone(),
                    NoteStatus::Committed {
                        block_height: inclusion_proof.origin().block_num as u64,
                    },
                    Some(*note.metadata()),
                    Some(inclusion_proof),
                    details,
                    false,
                    None,
                )
            },
        };
        let id = note.id();

        maybe_await!(self
            .store
            .insert_input_note(note)
            .map_err(<StoreError as Into<ClientError>>::into))?;
        Ok(id)
    }

    /// Compiles the provided program into a [NoteScript] and checks (to the extent possible) if
    /// the specified note program could be executed against all accounts with the specified
    /// interfaces.
    pub fn compile_note_script(
        &self,
        note_script_ast: ProgramAst,
        target_account_procs: Vec<ScriptTarget>,
    ) -> Result<NoteScript, ClientError> {
        self.tx_executor
            .compile_note_script(note_script_ast, target_account_procs)
            .map_err(ClientError::TransactionExecutorError)
    }
}

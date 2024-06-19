use miden_objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    crypto::rand::FeltRng,
    notes::{NoteId, NoteInclusionProof, NoteScript},
};
use miden_tx::{auth::TransactionAuthenticator, ScriptTarget};
use tracing::info;
use winter_maybe_async::{maybe_async, maybe_await};

use super::{note_screener::NoteConsumability, rpc::NodeRpcClient, Client};
use crate::{
    client::NoteScreener,
    errors::ClientError,
    store::{InputNoteRecord, NoteFilter, OutputNoteRecord, Store},
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
    pub async fn import_input_note(
        &mut self,
        note: InputNoteRecord,
        verify: bool,
    ) -> Result<(), ClientError> {
        if !verify {
            return maybe_await!(self.store.insert_input_note(&note)).map_err(|err| err.into());
        }

        // Verify that note exists in chain
        let mut chain_notes = self.rpc_api.get_notes_by_id(&[note.id()]).await?;
        if chain_notes.is_empty() {
            return Err(ClientError::ExistenceVerificationError(note.id()));
        }

        let note_details = chain_notes.pop().expect("chain_notes should have at least one element");
        let inclusion_details = note_details.inclusion_details();

        // If the note exists in the chain and the client is synced to a height equal or
        // greater than the note's creation block, get MMR and block header data for the
        // note's block. Additionally create the inclusion proof if none is provided.
        let inclusion_proof = if maybe_await!(self.get_sync_height())?
            >= inclusion_details.block_num
        {
            // Add the inclusion proof to the imported note
            info!("Requesting MMR data for past block num {}", inclusion_details.block_num);
            let mut current_partial_mmr = maybe_await!(self.build_current_partial_mmr(true))?;
            let block_header = self
                .get_and_store_authenticated_block(
                    inclusion_details.block_num,
                    &mut current_partial_mmr,
                )
                .await?;

            let built_inclusion_proof = NoteInclusionProof::new(
                inclusion_details.block_num,
                block_header.sub_hash(),
                block_header.note_root(),
                inclusion_details.note_index.into(),
                inclusion_details.merkle_path.clone(),
            )?;

            // If the imported note already provides an inclusion proof, check that
            // it equals the one we constructed from node data.
            if let Some(proof) = note.inclusion_proof() {
                if proof != &built_inclusion_proof {
                    return Err(ClientError::NoteImportError(
                        "Constructed inclusion proof does not equal the provided one".to_string(),
                    ));
                }
            }

            Some(built_inclusion_proof)
        } else {
            None
        };

        let note = InputNoteRecord::new(
            note.id(),
            note.recipient(),
            note.assets().clone(),
            note.status(),
            note.metadata().copied(),
            inclusion_proof,
            note.details().clone(),
        );

        maybe_await!(self.store.insert_input_note(&note)).map_err(|err| err.into())
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

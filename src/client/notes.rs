use miden_objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    crypto::rand::FeltRng,
    notes::{NoteId, NoteScript},
};
use miden_tx::ScriptTarget;

use super::{note_screener::NoteRelevance, rpc::NodeRpcClient, Client};
use crate::{
    client::NoteScreener,
    errors::ClientError,
    store::{InputNoteRecord, NoteFilter, OutputNoteRecord, Store},
};

// TYPES
// --------------------------------------------------------------------------------------------
/// Contains information about a note that can be consumed
pub struct ConsumableNote {
    /// The consumable note
    pub note: InputNoteRecord,
    /// Stores which accounts can consume the note and it's relevance
    pub relevances: Vec<(AccountId, NoteRelevance)>,
}

impl<N: NodeRpcClient, R: FeltRng, S: Store> Client<N, R, S> {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    pub fn get_input_notes(&self, filter: NoteFilter) -> Result<Vec<InputNoteRecord>, ClientError> {
        self.store.get_input_notes(filter).map_err(|err| err.into())
    }

    /// Returns input notes that are able to be consumed by the account_id.
    ///
    /// If account_id is None then all consumable input notes are returned.
    pub fn get_consumable_notes(
        &self,
        account_id: Option<AccountId>,
    ) -> Result<Vec<ConsumableNote>, ClientError> {
        let commited_notes = self.store.get_input_notes(NoteFilter::Committed)?;

        let note_screener = NoteScreener::new(self.store.as_ref());

        let mut relevant_notes = Vec::new();
        for input_note in commited_notes {
            let account_relevance =
                note_screener.check_relevance(&input_note.clone().try_into()?)?;

            if account_relevance.is_empty() {
                continue;
            }

            relevant_notes.push(ConsumableNote {
                note: input_note,
                relevances: account_relevance,
            });
        }

        if let Some(account_id) = account_id {
            relevant_notes.retain(|note| note.relevances.iter().any(|(id, _)| *id == account_id));
        }

        Ok(relevant_notes)
    }

    /// Returns the input note with the specified hash.
    pub fn get_input_note(&self, note_id: NoteId) -> Result<InputNoteRecord, ClientError> {
        Ok(self
            .store
            .get_input_notes(NoteFilter::Unique(note_id))?
            .pop()
            .expect("The vector always has one element for NoteFilter::Unique"))
    }

    // OUTPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns output notes managed by this client.
    pub fn get_output_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, ClientError> {
        self.store.get_output_notes(filter).map_err(|err| err.into())
    }

    /// Returns the output note with the specified hash.
    pub fn get_output_note(&self, note_id: NoteId) -> Result<OutputNoteRecord, ClientError> {
        Ok(self
            .store
            .get_output_notes(NoteFilter::Unique(note_id))?
            .pop()
            .expect("The vector always has one element for NoteFilter::Unique"))
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store.
    pub fn import_input_note(&mut self, note: InputNoteRecord) -> Result<(), ClientError> {
        self.store.insert_input_note(&note).map_err(|err| err.into())
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

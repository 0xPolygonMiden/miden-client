use miden_objects::{accounts::AccountId, crypto::rand::FeltRng, notes::NoteId};

use super::{note_screener::NoteRelevance, rpc::NodeRpcClient, Client};
use crate::{
    client::NoteScreener,
    errors::ClientError,
    store::{InputNoteRecord, NoteFilter, Store},
};

// TYPES
// --------------------------------------------------------------------------------------------
pub type ConsumableNote = (InputNoteRecord, Vec<(AccountId, NoteRelevance)>);

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

        let note_screener = NoteScreener::new(&self.store);

        let mut relevant_notes = Vec::new();
        for input_note in commited_notes {
            let account_relevance =
                note_screener.check_relevance(&input_note.clone().try_into()?)?;

            if account_relevance.is_empty() {
                continue;
            }

            relevant_notes.push((input_note, account_relevance));
        }

        if let Some(account_id) = account_id {
            relevant_notes.retain(|(_input_note, relevance)| {
                relevance.iter().any(|(id, _)| *id == account_id)
            });
        }

        Ok(relevant_notes)
    }

    /// Returns the input note with the specified hash.
    pub fn get_input_note(&self, note_id: NoteId) -> Result<InputNoteRecord, ClientError> {
        self.store.get_input_note(note_id).map_err(|err| err.into())
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store.
    pub fn import_input_note(&mut self, note: InputNoteRecord) -> Result<(), ClientError> {
        self.store.insert_input_note(&note).map_err(|err| err.into())
    }
}

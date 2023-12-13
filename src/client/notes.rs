use super::Client;

use crate::{errors::ClientError, store::notes::InputNoteFilter};
use objects::{notes::RecordedNote, Digest};

impl Client {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    pub fn get_input_notes(
        &self,
        filter: InputNoteFilter,
    ) -> Result<Vec<RecordedNote>, ClientError> {
        self.store.get_input_notes(filter).map_err(|err| err.into())
    }

    /// Returns the input note with the specified hash.
    pub fn get_input_note(&self, hash: Digest) -> Result<RecordedNote, ClientError> {
        self.store
            .get_input_note_by_hash(hash)
            .map_err(|err| err.into())
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Inserts a new input note into the client's store.
    pub fn insert_input_note(&mut self, note: RecordedNote) -> Result<(), ClientError> {
        self.store
            .insert_input_note(&note)
            .map_err(|err| err.into())
    }

    // TODO: add methods for retrieving note and transaction info, and for creating/executing
    // transaction
}

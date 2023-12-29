use super::Client;

use crate::{
    errors::ClientError,
    store::notes::{InputNoteFilter, InputNoteRecord},
};
use objects::Digest;

impl Client {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    pub fn get_input_notes(
        &self,
        filter: InputNoteFilter,
    ) -> Result<Vec<InputNoteRecord>, ClientError> {
        self.store.get_input_notes(filter).map_err(|err| err.into())
    }

    /// Returns the input note with the specified hash.
    pub fn get_input_note(&self, hash: Digest) -> Result<InputNoteRecord, ClientError> {
        self.store
            .get_input_note_by_hash(hash)
            .map_err(|err| err.into())
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store.
    pub fn import_input_note(&mut self, note: InputNoteRecord) -> Result<(), ClientError> {
        self.store
            .insert_input_note(&note)
            .map_err(|err| err.into())
    }
}

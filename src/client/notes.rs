use super::Client;

use crate::{
    errors::ClientError,
    store::notes::{InputNoteFilter, NoteType},
};
use objects::{
    notes::{Note, RecordedNote},
    Digest,
};

impl Client {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    pub fn get_input_notes(&self, filter: InputNoteFilter) -> Result<Vec<NoteType>, ClientError> {
        self.store.get_input_notes(filter).map_err(|err| err.into())
    }

    /// Returns input notes managed by this client.
    pub fn get_recorded_notes(&self) -> Result<Vec<RecordedNote>, ClientError> {
        self.store.get_recorded_notes().map_err(|err| err.into())
    }

    /// Returns the input note with the specified hash.
    pub fn get_input_note(&self, hash: Digest) -> Result<NoteType, ClientError> {
        self.store
            .get_input_note_by_hash(hash)
            .map_err(|err| err.into())
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Inserts a new input note into the client's store.
    pub fn import_input_note(&mut self, note: RecordedNote) -> Result<(), ClientError> {
        self.store
            .insert_input_note(&note)
            .map_err(|err| err.into())
    }

    /// Inserts a new pending note into the client's store.
    pub fn insert_pending_note(&mut self, note: Note) -> Result<(), ClientError> {
        self.store
            .insert_pending_note(&note)
            .map_err(|err| err.into())
    }
}

use miden_objects::notes::NoteId;

use super::{rpc::NodeRpcClient, Client};
use crate::{
    errors::ClientError,
    store::{InputNoteRecord, NoteFilter, Store},
};

impl<N: NodeRpcClient, S: Store> Client<N, S> {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    pub fn get_input_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, ClientError> {
        self.store.get_input_notes(filter).map_err(|err| err.into())
    }

    /// Returns the input note with the specified hash.
    pub fn get_input_note(
        &self,
        note_id: NoteId,
    ) -> Result<InputNoteRecord, ClientError> {
        self.store.get_input_note(note_id).map_err(|err| err.into())
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store.
    pub fn import_input_note(
        &mut self,
        note: InputNoteRecord,
    ) -> Result<(), ClientError> {
        self.store.insert_input_note(&note).map_err(|err| err.into())
    }
}

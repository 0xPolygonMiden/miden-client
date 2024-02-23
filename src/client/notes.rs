use super::{rpc::NodeRpcClient, Client};

use crate::{
    errors::ClientError,
    store::{NoteFilter, NoteRecord},
};
use miden_tx::DataStore;
use objects::notes::NoteId;

impl<N: NodeRpcClient, D: DataStore> Client<N, D> {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    pub fn get_input_notes(&self, filter: NoteFilter) -> Result<Vec<NoteRecord>, ClientError> {
        self.store.get_input_notes(filter).map_err(|err| err.into())
    }

    /// Returns the input note with the specified hash.
    pub fn get_input_note(&self, note_id: NoteId) -> Result<NoteRecord, ClientError> {
        self.store.get_input_note(note_id).map_err(|err| err.into())
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store.
    pub fn import_input_note(&mut self, note: NoteRecord) -> Result<(), ClientError> {
        self.store
            .insert_input_note(&note)
            .map_err(|err| err.into())
    }
}

use miden_objects::{crypto::rand::FeltRng, notes::NoteId};

use crate::native_code::store::NoteFilter;

use super::{
    errors::ClientError, 
    rpc::NodeRpcClient, 
    store::{note_record::InputNoteRecord, Store}, 
    Client
};

impl<N: NodeRpcClient, R: FeltRng, S: Store> Client<N, R, S> {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    pub async fn get_input_notes(&self,filter: NoteFilter) -> Result<Vec<InputNoteRecord>, ClientError> {
        self.store.get_input_notes(filter).await.map_err(|err| err.into())
    }

    /// Returns the input note with the specified hash.
    pub async fn get_input_note(&self,note_id: NoteId,) -> Result<InputNoteRecord, ClientError> {
        self.store.get_input_note(note_id).await.map_err(|err| err.into())
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store.
    pub async fn import_input_note(&mut self, note: InputNoteRecord) -> Result<(), ClientError> {
        self.store.insert_input_note(&note).await.map_err(|err| err.into())
    }
}
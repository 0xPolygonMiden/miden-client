use miden_objects::{crypto::rand::FeltRng, notes::NoteId};

use crate::native_code::store::NoteFilter;

use super::{
    errors::StoreError, rpc::NodeRpcClient, store::{note_record::InputNoteRecord, Store}, Client // TODO: Add AuthInfo
};

impl<N: NodeRpcClient, R: FeltRng, S: Store> Client<N, R, S> {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    pub async fn get_input_notes(
        &mut self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.store.get_input_notes(filter).await
    }

    /// Returns the input note with the specified hash.
    pub async fn get_input_note(
        &mut self,
        note_id: NoteId,
    ) -> Result<InputNoteRecord, StoreError> {
        self.store.get_input_note(note_id).await
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store.
    pub async fn import_input_note(
        &mut self,
        note: InputNoteRecord,
    ) -> Result<(), StoreError> {
        self.store.insert_input_note(&note).await
    }
}
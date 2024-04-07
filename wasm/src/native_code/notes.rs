use crate::native_code::store::NativeNoteFilter;

use super::{
    rpc::NodeRpcClient, 
    Client, 
    store::Store // TODO: Add AuthInfo
};

impl<N: NodeRpcClient, S: Store> Client<N, S> {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    pub async fn get_input_notes(
        &self,
        filter: NativeNoteFilter,
    ) -> String { // TODO: Replace with Result<Vec<InputNoteRecord>, ()>
        //self.store.get_input_notes(filter).map_err(|err| err.into())

        "Called get_input_notes".to_string()
    }

    /// Returns the input note with the specified hash.
    pub async fn get_input_note(
        &self,
        //note_id: NoteId,
    ) -> String { // TODO: Replace with Result<InputNoteRecord, ()>
        //self.store.get_input_note(note_id).map_err(|err| err.into())

        "Called get_input_note".to_string()
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store.
    pub async fn import_input_note(
        &mut self,
        //note: InputNoteRecord,
    ) -> String { // TODO: Replace with Result<(), ()>
        //self.store.insert_input_note(&note).map_err(|err| err.into())

        "Called import_input_note".to_string()
    }
}
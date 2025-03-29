use miden_client::store::InputNoteRecord as NativeInputNoteRecord;
use wasm_bindgen::prelude::*;

use super::{
    input_note_state::InputNoteState, note_details::NoteDetails, note_id::NoteId,
    note_inclusion_proof::NoteInclusionProof, note_metadata::NoteMetadata,
};

#[derive(Clone)]
#[wasm_bindgen]
pub struct InputNoteRecord(NativeInputNoteRecord);

#[wasm_bindgen]
impl InputNoteRecord {
    pub fn id(&self) -> NoteId {
        self.0.id().into()
    }

    pub fn state(&self) -> InputNoteState {
        self.0.state().into()
    }

    pub fn details(&self) -> NoteDetails {
        self.0.details().into()
    }

    pub fn metadata(&self) -> Option<NoteMetadata> {
        self.0.metadata().map(Into::into)
    }

    #[wasm_bindgen(js_name = "inclusionProof")]
    pub fn inclusion_proof(&self) -> Option<NoteInclusionProof> {
        self.0.inclusion_proof().map(Into::into)
    }

    #[wasm_bindgen(js_name = "consumerTransactionId")]
    pub fn consumer_transaction_id(&self) -> Option<String> {
        self.0.consumer_transaction_id().map(ToString::to_string)
    }

    pub fn nullifier(&self) -> String {
        self.0.nullifier().to_hex()
    }

    #[wasm_bindgen(js_name = "isAuthenticated")]
    pub fn is_authenticated(&self) -> bool {
        self.0.is_authenticated()
    }

    #[wasm_bindgen(js_name = "isConsumed")]
    pub fn is_consumed(&self) -> bool {
        self.0.is_consumed()
    }

    #[wasm_bindgen(js_name = "isProcessing")]
    pub fn is_processing(&self) -> bool {
        self.0.is_processing()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeInputNoteRecord> for InputNoteRecord {
    fn from(native_note: NativeInputNoteRecord) -> Self {
        InputNoteRecord(native_note)
    }
}

impl From<&NativeInputNoteRecord> for InputNoteRecord {
    fn from(native_note: &NativeInputNoteRecord) -> Self {
        InputNoteRecord(native_note.clone())
    }
}

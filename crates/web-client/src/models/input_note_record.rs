use miden_client::store::InputNoteRecord as NativeInputNoteRecord;
use wasm_bindgen::prelude::*;

use super::{
    input_note_state::InputNoteState, note_details::NoteDetails, note_id::NoteId,
    note_metadata::NoteMetadata,
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
        match self.0.metadata() {
            Some(metadata) => Some(metadata.into()),
            None => None,
        }
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

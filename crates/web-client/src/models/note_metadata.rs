use miden_objects::note::NoteMetadata as NativeNoteMetadata;
use wasm_bindgen::prelude::*;

use super::{
    account_id::AccountId, felt::Felt, note_execution_hint::NoteExecutionHint, note_tag::NoteTag,
    note_type::NoteType,
};

#[derive(Clone, Copy)]
#[wasm_bindgen]
pub struct NoteMetadata(NativeNoteMetadata);

#[wasm_bindgen]
impl NoteMetadata {
    #[wasm_bindgen(constructor)]
    pub fn new(
        sender: &AccountId,
        note_type: NoteType,
        note_tag: &NoteTag,
        note_execution_hint: &NoteExecutionHint,
        aux: Option<Felt>, // Create an OptionFelt type so user has choice to consume or not
    ) -> NoteMetadata {
        let native_note_metadata = NativeNoteMetadata::new(
            sender.into(),
            note_type.into(),
            note_tag.into(),
            note_execution_hint.into(),
            aux.map_or(miden_objects::Felt::default(), Into::into),
        )
        .unwrap();
        NoteMetadata(native_note_metadata)
    }

    pub fn sender(&self) -> AccountId {
        self.0.sender().into()
    }

    pub fn tag(&self) -> NoteTag {
        self.0.tag().into()
    }

    #[wasm_bindgen(js_name = "noteType")]
    pub fn note_type(&self) -> NoteType {
        self.0.note_type().into()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeNoteMetadata> for NoteMetadata {
    fn from(native_note_metadata: NativeNoteMetadata) -> Self {
        NoteMetadata(native_note_metadata)
    }
}

impl From<&NativeNoteMetadata> for NoteMetadata {
    fn from(native_note_metadata: &NativeNoteMetadata) -> Self {
        NoteMetadata(*native_note_metadata)
    }
}

impl From<NoteMetadata> for NativeNoteMetadata {
    fn from(note_metadata: NoteMetadata) -> Self {
        note_metadata.0
    }
}

impl From<&NoteMetadata> for NativeNoteMetadata {
    fn from(note_metadata: &NoteMetadata) -> Self {
        note_metadata.0
    }
}

use miden_objects::{
    accounts::AccountId as NativeAccountId,
    Felt as NativeFelt,
    notes::{NoteExecutionHint as NativeNoteExecutionHint, NoteMetadata as NativeNoteMetadata, NoteTag as NativeNoteTag, NoteType as NativeNoteType}
};
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
        note_type: &NoteType,
        note_tag: &NoteTag,
        note_execution_hint: &NoteExecutionHint,
        aux: Option<Felt>, // Create an OptionFelt type so user has choice to consume or not
    ) -> NoteMetadata {
        let native_sender: NativeAccountId = sender.into();
        let native_note_type: NativeNoteType = note_type.into();
        let native_tag: NativeNoteTag = note_tag.into();
        let native_execution_hint: NativeNoteExecutionHint = note_execution_hint.into();
        let native_aux: NativeFelt = match aux {
            Some(felt) => felt.into(),
            None => Default::default(),
        };

        let native_note_metadata = NativeNoteMetadata::new(
            native_sender,
            native_note_type,
            native_tag,
            native_execution_hint,
            native_aux,
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

    pub fn note_type(&self) -> NoteType {
        self.0.note_type().into()
    }
}

// Conversions

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

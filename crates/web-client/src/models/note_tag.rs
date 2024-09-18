use miden_objects::{
    accounts::AccountId as NativeAccountId,
    notes::{NoteExecutionMode as NativeNoteExecutionMode, NoteTag as NativeNoteTag},
};
use wasm_bindgen::prelude::*;

use super::{account_id::AccountId, note_execution_mode::NoteExecutionMode};

#[derive(Clone, Copy)]
#[wasm_bindgen]
pub struct NoteTag(NativeNoteTag);

#[wasm_bindgen]
impl NoteTag {
    pub fn from_account_id(account_id: &AccountId, execution: &NoteExecutionMode) -> NoteTag {
        let native_account_id: NativeAccountId = account_id.into();
        let native_execution: NativeNoteExecutionMode = execution.into();
        let native_note_tag =
            NativeNoteTag::from_account_id(native_account_id, native_execution).unwrap();
        NoteTag(native_note_tag)
    }

    pub fn for_public_use_case(
        use_case_id: u16,
        payload: u16,
        execution: &NoteExecutionMode,
    ) -> NoteTag {
        let native_execution: NativeNoteExecutionMode = execution.into();
        let native_note_tag =
            NativeNoteTag::for_public_use_case(use_case_id, payload, native_execution).unwrap();
        NoteTag(native_note_tag)
    }

    pub fn for_local_use_case(use_case_id: u16, payload: u16) -> NoteTag {
        let native_note_tag = NativeNoteTag::for_local_use_case(use_case_id, payload).unwrap();
        NoteTag(native_note_tag)
    }

    pub fn is_single_target(&self) -> bool {
        self.0.is_single_target()
    }

    pub fn execution_hint(&self) -> NoteExecutionMode {
        self.0.execution_hint().into()
    }
}

// Conversions

impl From<NativeNoteTag> for NoteTag {
    fn from(native_note_tag: NativeNoteTag) -> Self {
        NoteTag(native_note_tag)
    }
}

impl From<&NativeNoteTag> for NoteTag {
    fn from(native_note_tag: &NativeNoteTag) -> Self {
        NoteTag(*native_note_tag)
    }
}

impl From<NoteTag> for NativeNoteTag {
    fn from(note_tag: NoteTag) -> Self {
        note_tag.0
    }
}

impl From<&NoteTag> for NativeNoteTag {
    fn from(note_tag: &NoteTag) -> Self {
        note_tag.0
    }
}

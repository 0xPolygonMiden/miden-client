use miden_client::notes::{scripts as native_scripts, NoteScript as NativeNoteScript};
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteScript(NativeNoteScript);

#[wasm_bindgen]
impl NoteScript {
    pub fn p2id() -> Self {
        native_scripts::p2id().into()
    }

    pub fn p2idr() -> Self {
        native_scripts::p2idr().into()
    }

    pub fn swap() -> Self {
        native_scripts::swap().into()
    }
}
// CONVERSIONS
// ================================================================================================

impl From<NativeNoteScript> for NoteScript {
    fn from(native_note_script: NativeNoteScript) -> Self {
        NoteScript(native_note_script)
    }
}

impl From<&NativeNoteScript> for NoteScript {
    fn from(native_note_script: &NativeNoteScript) -> Self {
        NoteScript(native_note_script.clone())
    }
}

impl From<NoteScript> for NativeNoteScript {
    fn from(note_script: NoteScript) -> Self {
        note_script.0
    }
}

impl From<&NoteScript> for NativeNoteScript {
    fn from(note_script: &NoteScript) -> Self {
        note_script.0.clone()
    }
}

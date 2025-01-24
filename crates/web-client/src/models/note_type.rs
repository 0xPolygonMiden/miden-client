use miden_objects::note::NoteType as NativeNoteType;
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy)]
#[wasm_bindgen]
pub struct NoteType(NativeNoteType);

#[wasm_bindgen]
impl NoteType {
    pub fn private() -> NoteType {
        NoteType(NativeNoteType::Private)
    }

    pub fn public() -> NoteType {
        NoteType(NativeNoteType::Public)
    }

    pub fn encrypted() -> NoteType {
        NoteType(NativeNoteType::Encrypted)
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeNoteType> for NoteType {
    fn from(native_note_type: NativeNoteType) -> Self {
        NoteType(native_note_type)
    }
}

impl From<&NativeNoteType> for NoteType {
    fn from(native_note_type: &NativeNoteType) -> Self {
        NoteType(*native_note_type)
    }
}

impl From<NoteType> for NativeNoteType {
    fn from(note_type: NoteType) -> Self {
        note_type.0
    }
}

impl From<&NoteType> for NativeNoteType {
    fn from(note_type: &NoteType) -> Self {
        note_type.0
    }
}

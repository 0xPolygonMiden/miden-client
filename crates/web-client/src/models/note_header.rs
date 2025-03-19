use miden_objects::note::NoteHeader as NativeNoteHeader;
use wasm_bindgen::prelude::*;

use super::{note_id::NoteId, note_metadata::NoteMetadata, rpo_digest::RpoDigest};

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteHeader(NativeNoteHeader);

#[wasm_bindgen]
impl NoteHeader {
    // TODO: new()

    pub fn id(&self) -> NoteId {
        self.0.id().into()
    }

    pub fn metadata(&self) -> NoteMetadata {
        self.0.metadata().into()
    }

    pub fn commitment(&self) -> RpoDigest {
        self.0.commitment().into()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeNoteHeader> for NoteHeader {
    fn from(native_note_header: NativeNoteHeader) -> Self {
        NoteHeader(native_note_header)
    }
}

impl From<&NativeNoteHeader> for NoteHeader {
    fn from(native_note_header: &NativeNoteHeader) -> Self {
        NoteHeader(*native_note_header)
    }
}

impl From<NoteHeader> for NativeNoteHeader {
    fn from(note_header: NoteHeader) -> Self {
        note_header.0
    }
}

impl From<&NoteHeader> for NativeNoteHeader {
    fn from(note_header: &NoteHeader) -> Self {
        note_header.0
    }
}

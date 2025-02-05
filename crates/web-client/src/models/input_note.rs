use miden_objects::transaction::InputNote as NativeInputNote;
use wasm_bindgen::prelude::*;

use super::{
    note::Note, note_id::NoteId, note_inclusion_proof::NoteInclusionProof,
    note_location::NoteLocation,
};

#[derive(Clone)]
#[wasm_bindgen]
pub struct InputNote(NativeInputNote);

#[wasm_bindgen]
impl InputNote {
    // TODO: authenticated constructor

    // TODO: unauthenticated constructor

    pub fn id(&self) -> NoteId {
        self.0.id().into()
    }

    pub fn note(&self) -> Note {
        self.0.note().into()
    }

    pub fn proof(&self) -> Option<NoteInclusionProof> {
        self.0.proof().map(Into::into)
    }

    pub fn location(&self) -> Option<NoteLocation> {
        self.0.location().map(Into::into)
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeInputNote> for InputNote {
    fn from(native_note: NativeInputNote) -> Self {
        InputNote(native_note)
    }
}

impl From<&NativeInputNote> for InputNote {
    fn from(native_note: &NativeInputNote) -> Self {
        InputNote(native_note.clone())
    }
}

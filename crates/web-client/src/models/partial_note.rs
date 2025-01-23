use miden_objects::note::PartialNote as NativePartialNote;
use wasm_bindgen::prelude::*;

use super::{
    note_assets::NoteAssets, note_id::NoteId, note_metadata::NoteMetadata, rpo_digest::RpoDigest,
};

#[derive(Clone)]
#[wasm_bindgen]
pub struct PartialNote(NativePartialNote);

#[wasm_bindgen]
impl PartialNote {
    // TODO: new

    pub fn id(&self) -> NoteId {
        self.0.id().into()
    }

    pub fn metadata(&self) -> NoteMetadata {
        self.0.metadata().into()
    }

    pub fn recipient_digest(&self) -> RpoDigest {
        self.0.recipient_digest().into()
    }

    pub fn assets(&self) -> NoteAssets {
        self.0.assets().into()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativePartialNote> for PartialNote {
    fn from(native_note: NativePartialNote) -> Self {
        PartialNote(native_note)
    }
}

impl From<&NativePartialNote> for PartialNote {
    fn from(native_note: &NativePartialNote) -> Self {
        PartialNote(native_note.clone())
    }
}

impl From<PartialNote> for NativePartialNote {
    fn from(note: PartialNote) -> Self {
        note.0
    }
}

impl From<&PartialNote> for NativePartialNote {
    fn from(note: &PartialNote) -> Self {
        note.0.clone()
    }
}

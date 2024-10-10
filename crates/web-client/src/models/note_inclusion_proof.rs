use miden_objects::notes::NoteInclusionProof as NativeNoteInclusionProof;
use wasm_bindgen::prelude::*;

use super::{merkle_path::MerklePath, note_location::NoteLocation};

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteInclusionProof(NativeNoteInclusionProof);

#[wasm_bindgen]
impl NoteInclusionProof {
    pub fn location(&self) -> NoteLocation {
        self.0.location().into()
    }

    pub fn note_path(&self) -> MerklePath {
        self.0.note_path().into()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeNoteInclusionProof> for NoteInclusionProof {
    fn from(native_proof: NativeNoteInclusionProof) -> Self {
        NoteInclusionProof(native_proof)
    }
}

impl From<&NativeNoteInclusionProof> for NoteInclusionProof {
    fn from(native_proof: &NativeNoteInclusionProof) -> Self {
        NoteInclusionProof(native_proof.clone())
    }
}

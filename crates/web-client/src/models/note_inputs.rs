use miden_objects::{notes::NoteInputs as NativeNoteInputs, Felt as NativeFelt};
use wasm_bindgen::prelude::*;

use super::felt::{Felt, FeltArray};

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteInputs(NativeNoteInputs);

#[wasm_bindgen]
impl NoteInputs {
    #[wasm_bindgen(constructor)]
    pub fn new(felt_array: &FeltArray) -> NoteInputs {
        let native_felts = felt_array.into();
        let native_note_inputs = NativeNoteInputs::new(native_felts).unwrap();
        NoteInputs(native_note_inputs)
    }
}

// Conversions

impl From<NoteInputs> for NativeNoteInputs {
    fn from(note_inputs: NoteInputs) -> Self {
        note_inputs.0
    }
}

impl From<&NoteInputs> for NativeNoteInputs {
    fn from(note_inputs: &NoteInputs) -> Self {
        note_inputs.0.clone()
    }
}

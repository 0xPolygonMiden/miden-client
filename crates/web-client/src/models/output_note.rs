use miden_objects::{notes::Note as NativeNote, transaction::OutputNote as NativeOutputNote};
use wasm_bindgen::prelude::*;

use super::note::Note;

#[derive(Clone)]
#[wasm_bindgen]
pub struct OutputNote(NativeOutputNote);

#[wasm_bindgen]
impl OutputNote {
    pub fn full(note: &Note) -> OutputNote {
        let native_note: NativeNote = note.into();
        OutputNote(NativeOutputNote::Full(native_note))
    }
}

// Conversions

impl From<OutputNote> for NativeOutputNote {
    fn from(output_note: OutputNote) -> Self {
        output_note.0
    }
}

impl From<&OutputNote> for NativeOutputNote {
    fn from(output_note: &OutputNote) -> Self {
        output_note.0.clone()
    }
}

#[derive(Clone)]
#[wasm_bindgen]
pub struct OutputNotesArray(Vec<OutputNote>);

#[wasm_bindgen]
impl OutputNotesArray {
    #[wasm_bindgen(constructor)]
    pub fn new(output_notes_array: Option<Vec<OutputNote>>) -> OutputNotesArray {
        let output_notes = output_notes_array.unwrap_or_default();
        OutputNotesArray(output_notes)
    }

    pub fn append(&mut self, output_note: &OutputNote) {
        self.0.push(output_note.clone())
    }
}

// Conversions

impl From<OutputNotesArray> for Vec<NativeOutputNote> {
    fn from(output_notes_array: OutputNotesArray) -> Self {
        output_notes_array.0.into_iter().map(|output_note| output_note.into()).collect()
    }
}

impl From<&OutputNotesArray> for Vec<NativeOutputNote> {
    fn from(output_notes_array: &OutputNotesArray) -> Self {
        output_notes_array.0.iter().map(|output_note| output_note.into()).collect()
    }
}

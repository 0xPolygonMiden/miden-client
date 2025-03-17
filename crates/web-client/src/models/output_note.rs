use miden_objects::{
    note::{Note as NativeNote, NoteHeader as NativeNoteHeader, PartialNote as NativePartialNote},
    transaction::OutputNote as NativeOutputNote,
};
use wasm_bindgen::prelude::*;

use super::{
    note::Note, note_assets::NoteAssets, note_header::NoteHeader, note_id::NoteId,
    note_metadata::NoteMetadata, partial_note::PartialNote, rpo_digest::RpoDigest,
};

#[derive(Clone)]
#[wasm_bindgen]
pub struct OutputNote(NativeOutputNote);

#[wasm_bindgen]
impl OutputNote {
    pub fn full(note: &Note) -> OutputNote {
        let native_note: NativeNote = note.into();
        OutputNote(NativeOutputNote::Full(native_note))
    }

    pub fn partial(partial_note: &PartialNote) -> OutputNote {
        let native_partial_note: NativePartialNote = partial_note.into();
        OutputNote(NativeOutputNote::Partial(native_partial_note))
    }

    pub fn header(note_header: &NoteHeader) -> OutputNote {
        let native_note_header: NativeNoteHeader = note_header.into();
        OutputNote(NativeOutputNote::Header(native_note_header))
    }

    pub fn assets(&self) -> Option<NoteAssets> {
        self.0.assets().map(Into::into)
    }

    pub fn id(&self) -> NoteId {
        self.0.id().into()
    }

    #[wasm_bindgen(js_name = "recipientDigest")]
    pub fn recipient_digest(&self) -> Option<RpoDigest> {
        self.0.recipient_digest().map(Into::into)
    }

    pub fn metadata(&self) -> NoteMetadata {
        self.0.metadata().into()
    }

    #[must_use]
    pub fn shrink(&self) -> OutputNote {
        self.0.shrink().into()
    }

    #[wasm_bindgen(js_name = "intoFull")]
    pub fn into_full(self) -> Option<Note> {
        match self.0 {
            NativeOutputNote::Full(note) => Some(note.into()),
            _ => None,
        }
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeOutputNote> for OutputNote {
    fn from(native_output_note: NativeOutputNote) -> Self {
        OutputNote(native_output_note)
    }
}

impl From<&NativeOutputNote> for OutputNote {
    fn from(native_output_note: &NativeOutputNote) -> Self {
        OutputNote(native_output_note.clone())
    }
}

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
        self.0.push(output_note.clone());
    }
}

// CONVERSIONS
// ================================================================================================

impl From<OutputNotesArray> for Vec<NativeOutputNote> {
    fn from(output_notes_array: OutputNotesArray) -> Self {
        output_notes_array.0.into_iter().map(Into::into).collect()
    }
}

impl From<&OutputNotesArray> for Vec<NativeOutputNote> {
    fn from(output_notes_array: &OutputNotesArray) -> Self {
        output_notes_array.0.iter().map(Into::into).collect()
    }
}

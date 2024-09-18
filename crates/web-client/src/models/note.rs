use miden_objects::notes::Note as NativeNote;
use wasm_bindgen::prelude::*;

use super::{
    note_assets::NoteAssets, note_id::NoteId, note_metadata::NoteMetadata,
    note_recipient::NoteRecipient,
};

#[wasm_bindgen]
#[derive(Clone)]
pub struct Note(NativeNote);

#[wasm_bindgen]
impl Note {
    #[wasm_bindgen(constructor)]
    pub fn new(
        note_assets: &NoteAssets,
        note_metadata: &NoteMetadata,
        note_recipient: &NoteRecipient,
    ) -> Note {
        Note(NativeNote::new(note_assets.into(), note_metadata.into(), note_recipient.into()))
    }

    pub fn id(&self) -> NoteId {
        self.0.id().into()
    }

    pub fn metadata(&self) -> NoteMetadata {
        (*self.0.metadata()).into()
    }

    pub fn recipient(&self) -> NoteRecipient {
        self.0.recipient().clone().into()
    }
}

impl From<Note> for NativeNote {
    fn from(note: Note) -> Self {
        note.0
    }
}

impl From<&Note> for NativeNote {
    fn from(note: &Note) -> Self {
        note.0.clone()
    }
}

#[derive(Clone)]
#[wasm_bindgen]
pub struct NotesArray(Vec<Note>);

#[wasm_bindgen]
impl NotesArray {
    #[wasm_bindgen(constructor)]
    pub fn new(notes_array: Option<Vec<Note>>) -> NotesArray {
        let notes = notes_array.unwrap_or_default();
        NotesArray(notes)
    }

    pub fn push(&mut self, note: &Note) {
        self.0.push(note.clone())
    }
}

// Conversions

impl From<NotesArray> for Vec<NativeNote> {
    fn from(notes_array: NotesArray) -> Self {
        notes_array.0.into_iter().map(|note| note.into()).collect()
    }
}

impl From<&NotesArray> for Vec<NativeNote> {
    fn from(notes_array: &NotesArray) -> Self {
        notes_array.0.iter().map(|note| note.into()).collect()
    }
}

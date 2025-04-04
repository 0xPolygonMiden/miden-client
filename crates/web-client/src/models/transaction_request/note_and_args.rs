use miden_client::transaction::NoteArgs as NativeNoteArgs;
use miden_objects::note::Note as NativeNote;
use wasm_bindgen::prelude::*;

use crate::models::{note::Note, word::Word};

pub type NoteArgs = Word;

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteAndArgs {
    note: Note,
    args: Option<NoteArgs>,
}

#[wasm_bindgen]
impl NoteAndArgs {
    #[wasm_bindgen(constructor)]
    pub fn new(note: Note, args: Option<NoteArgs>) -> NoteAndArgs {
        NoteAndArgs { note, args }
    }
}

impl From<NoteAndArgs> for (NativeNote, Option<NativeNoteArgs>) {
    fn from(note_and_args: NoteAndArgs) -> Self {
        let native_note: NativeNote = note_and_args.note.into();
        let native_args: Option<NativeNoteArgs> = note_and_args.args.map(Into::into);
        (native_note, native_args)
    }
}

impl From<&NoteAndArgs> for (NativeNote, Option<NativeNoteArgs>) {
    fn from(note_and_args: &NoteAndArgs) -> Self {
        let native_note: NativeNote = note_and_args.note.clone().into();
        let native_args: Option<NativeNoteArgs> = note_and_args.args.clone().map(Into::into);
        (native_note, native_args)
    }
}

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteAndArgsArray(Vec<NoteAndArgs>);

#[wasm_bindgen]
impl NoteAndArgsArray {
    #[wasm_bindgen(constructor)]
    pub fn new(note_and_args: Option<Vec<NoteAndArgs>>) -> NoteAndArgsArray {
        let note_and_args = note_and_args.unwrap_or_default();
        NoteAndArgsArray(note_and_args)
    }

    pub fn push(&mut self, note_and_args: &NoteAndArgs) {
        self.0.push(note_and_args.clone());
    }
}

impl From<NoteAndArgsArray> for Vec<(NativeNote, Option<NativeNoteArgs>)> {
    fn from(note_and_args_array: NoteAndArgsArray) -> Self {
        note_and_args_array.0.into_iter().map(Into::into).collect()
    }
}

impl From<&NoteAndArgsArray> for Vec<(NativeNote, Option<NativeNoteArgs>)> {
    fn from(note_and_args_array: &NoteAndArgsArray) -> Self {
        note_and_args_array.0.iter().map(Into::into).collect()
    }
}

use miden_client::transaction::NoteArgs as NativeNoteArgs;
use miden_objects::note::NoteId as NativeNoteId;
use wasm_bindgen::prelude::*;

use crate::models::{note_id::NoteId, transaction_request::note_and_args::NoteArgs};

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteIdAndArgs {
    note_id: NoteId,
    args: Option<NoteArgs>,
}

#[wasm_bindgen]
impl NoteIdAndArgs {
    #[wasm_bindgen(constructor)]
    pub fn new(note_id: NoteId, args: Option<NoteArgs>) -> NoteIdAndArgs {
        NoteIdAndArgs { note_id, args }
    }
}

impl From<NoteIdAndArgs> for (NativeNoteId, Option<NativeNoteArgs>) {
    fn from(note_id_and_args: NoteIdAndArgs) -> Self {
        let native_note_id: NativeNoteId = note_id_and_args.note_id.into();
        let native_args: Option<NativeNoteArgs> = note_id_and_args.args.map(Into::into);
        (native_note_id, native_args)
    }
}

impl From<&NoteIdAndArgs> for (NativeNoteId, Option<NativeNoteArgs>) {
    fn from(note_id_and_args: &NoteIdAndArgs) -> Self {
        let native_note_id: NativeNoteId = note_id_and_args.note_id.clone().into();
        let native_args: Option<NativeNoteArgs> =
            note_id_and_args.args.clone().map(|args| args.clone().into());
        (native_note_id, native_args)
    }
}

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteIdAndArgsArray(Vec<NoteIdAndArgs>);

#[wasm_bindgen]
impl NoteIdAndArgsArray {
    #[wasm_bindgen(constructor)]
    pub fn new(note_id_and_args: Option<Vec<NoteIdAndArgs>>) -> NoteIdAndArgsArray {
        let note_id_and_args = note_id_and_args.unwrap_or_default();
        NoteIdAndArgsArray(note_id_and_args)
    }

    pub fn push(&mut self, note_id_and_args: &NoteIdAndArgs) {
        self.0.push(note_id_and_args.clone());
    }
}

impl From<NoteIdAndArgsArray> for Vec<(NativeNoteId, Option<NativeNoteArgs>)> {
    fn from(note_id_and_args_array: NoteIdAndArgsArray) -> Self {
        note_id_and_args_array.0.into_iter().map(Into::into).collect()
    }
}

impl From<&NoteIdAndArgsArray> for Vec<(NativeNoteId, Option<NativeNoteArgs>)> {
    fn from(note_id_and_args_array: &NoteIdAndArgsArray) -> Self {
        note_id_and_args_array.0.iter().map(Into::into).collect()
    }
}

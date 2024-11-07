use miden_objects::notes::NoteDetails as NativeNoteDetails;
use wasm_bindgen::prelude::*;

use super::{note_assets::NoteAssets, note_recipient::NoteRecipient};

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteDetails(NativeNoteDetails);

#[wasm_bindgen]
impl NoteDetails {
    #[wasm_bindgen(constructor)]
    pub fn new(note_assets: &NoteAssets, note_recipient: &NoteRecipient) -> NoteDetails {
        NoteDetails(NativeNoteDetails::new(note_assets.into(), note_recipient.into()))
    }
}

impl From<NoteDetails> for NativeNoteDetails {
    fn from(note_details: NoteDetails) -> Self {
        note_details.0
    }
}

impl From<&NoteDetails> for NativeNoteDetails {
    fn from(note_details: &NoteDetails) -> Self {
        note_details.0.clone()
    }
}

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteDetailsArray(Vec<NoteDetails>);

#[wasm_bindgen]
impl NoteDetailsArray {
    #[wasm_bindgen(constructor)]
    pub fn new(note_details_array: Option<Vec<NoteDetails>>) -> NoteDetailsArray {
        let note_details_array = note_details_array.unwrap_or_default();
        NoteDetailsArray(note_details_array)
    }

    pub fn push(&mut self, note_details: &NoteDetails) {
        self.0.push(note_details.clone());
    }
}

impl From<NoteDetailsArray> for Vec<NativeNoteDetails> {
    fn from(note_details_array: NoteDetailsArray) -> Self {
        note_details_array
            .0
            .into_iter()
            .map(|note_details| note_details.into())
            .collect()
    }
}

impl From<&NoteDetailsArray> for Vec<NativeNoteDetails> {
    fn from(note_details_array: &NoteDetailsArray) -> Self {
        note_details_array.0.iter().map(|note_details| note_details.into()).collect()
    }
}

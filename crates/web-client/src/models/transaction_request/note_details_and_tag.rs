use miden_objects::note::{NoteDetails as NativeNoteDetails, NoteTag as NativeNoteTag};
use wasm_bindgen::prelude::*;

use crate::models::{note_details::NoteDetails, note_tag::NoteTag};

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteDetailsAndTag {
    note_details: NoteDetails,
    tag: NoteTag,
}

#[wasm_bindgen]
impl NoteDetailsAndTag {
    #[wasm_bindgen(constructor)]
    pub fn new(note_details: NoteDetails, tag: NoteTag) -> NoteDetailsAndTag {
        NoteDetailsAndTag { note_details, tag }
    }
}

impl From<NoteDetailsAndTag> for (NativeNoteDetails, NativeNoteTag) {
    fn from(note_details_and_args: NoteDetailsAndTag) -> Self {
        let native_note_details: NativeNoteDetails = note_details_and_args.note_details.into();
        let native_tag: NativeNoteTag = note_details_and_args.tag.into();
        (native_note_details, native_tag)
    }
}

impl From<&NoteDetailsAndTag> for (NativeNoteDetails, NativeNoteTag) {
    fn from(note_details_and_args: &NoteDetailsAndTag) -> Self {
        let native_note_details: NativeNoteDetails =
            note_details_and_args.note_details.clone().into();
        let native_tag: NativeNoteTag = note_details_and_args.tag.into();
        (native_note_details, native_tag)
    }
}

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteDetailsAndTagArray(Vec<NoteDetailsAndTag>);

#[wasm_bindgen]
impl NoteDetailsAndTagArray {
    #[wasm_bindgen(constructor)]
    pub fn new(
        note_details_and_tag_array: Option<Vec<NoteDetailsAndTag>>,
    ) -> NoteDetailsAndTagArray {
        let note_details_and_tag_array = note_details_and_tag_array.unwrap_or_default();
        NoteDetailsAndTagArray(note_details_and_tag_array)
    }

    pub fn push(&mut self, note_details_and_tag: &NoteDetailsAndTag) {
        self.0.push(note_details_and_tag.clone());
    }
}

impl From<NoteDetailsAndTagArray> for Vec<(NativeNoteDetails, NativeNoteTag)> {
    fn from(note_details_and_tag_array: NoteDetailsAndTagArray) -> Self {
        note_details_and_tag_array.0.into_iter().map(Into::into).collect()
    }
}

impl From<&NoteDetailsAndTagArray> for Vec<(NativeNoteDetails, NativeNoteTag)> {
    fn from(note_details_and_tag_array: &NoteDetailsAndTagArray) -> Self {
        note_details_and_tag_array.0.iter().map(Into::into).collect()
    }
}

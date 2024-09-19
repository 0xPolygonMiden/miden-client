use miden_client::transactions::{
    NoteArgs as NativeNoteArgs, TransactionRequest as NativeTransactionRequest,
};
use miden_objects::{
    notes::{Note as NativeNote, NoteDetails as NativeNoteDetails, NoteId as NativeNoteId},
    transaction::{OutputNote as NativeOutputNote, TransactionScript as NativeTransactionScript},
    vm::AdviceMap as NativeAdviceMap,
};
use wasm_bindgen::prelude::*;

use super::{
    advice_map::AdviceMap,
    note::{Note, NotesArray},
    note_details::NoteDetailsArray,
    note_id::NoteId,
    output_note::OutputNotesArray,
    transaction_script::TransactionScript,
    word::Word,
};

// NoteAndArgs Helper Structs

pub type NoteArgs = Word;

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteAndArgs {
    note: Note,
    args: Option<NoteArgs>,
}

#[wasm_bindgen]
impl NoteAndArgs {
    pub fn new(note: Note, args: Option<NoteArgs>) -> NoteAndArgs {
        NoteAndArgs { note, args }
    }
}

impl From<NoteAndArgs> for (NativeNote, Option<NativeNoteArgs>) {
    fn from(note_and_args: NoteAndArgs) -> Self {
        let native_note: NativeNote = note_and_args.note.into();
        let native_args: Option<NativeNoteArgs> = note_and_args.args.map(|args| args.into());
        (native_note, native_args)
    }
}

impl From<&NoteAndArgs> for (NativeNote, Option<NativeNoteArgs>) {
    fn from(note_and_args: &NoteAndArgs) -> Self {
        let native_note: NativeNote = note_and_args.note.clone().into();
        let native_args: Option<NativeNoteArgs> =
            note_and_args.args.clone().map(|args| args.into());
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
        note_and_args_array
            .0
            .into_iter()
            .map(|note_and_args| note_and_args.into())
            .collect()
    }
}

impl From<&NoteAndArgsArray> for Vec<(NativeNote, Option<NativeNoteArgs>)> {
    fn from(note_and_args_array: &NoteAndArgsArray) -> Self {
        note_and_args_array.0.iter().map(|note_and_args| note_and_args.into()).collect()
    }
}

// NoteIdAndArgs Helper Structs

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
        let native_args: Option<NativeNoteArgs> = note_id_and_args.args.map(|args| args.into());
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
        note_id_and_args_array
            .0
            .into_iter()
            .map(|note_id_and_args| note_id_and_args.into())
            .collect()
    }
}

impl From<&NoteIdAndArgsArray> for Vec<(NativeNoteId, Option<NativeNoteArgs>)> {
    fn from(note_id_and_args_array: &NoteIdAndArgsArray) -> Self {
        note_id_and_args_array
            .0
            .iter()
            .map(|note_id_and_args| note_id_and_args.into())
            .collect()
    }
}

// Transaction Request

#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionRequest(NativeTransactionRequest);

#[wasm_bindgen]
impl TransactionRequest {
    #[wasm_bindgen(constructor)]
    pub fn new() -> TransactionRequest {
        let native_transaction_request = NativeTransactionRequest::new();
        TransactionRequest(native_transaction_request)
    }

    pub fn with_unauthenticated_input_notes(mut self, notes: &NoteAndArgsArray) -> Self {
        let native_note_and_note_args: Vec<(NativeNote, Option<NativeNoteArgs>)> = notes.into();
        self.0 = self.0.clone().with_unauthenticated_input_notes(native_note_and_note_args);
        self
    }

    pub fn with_authenticated_input_notes(mut self, notes: &NoteIdAndArgsArray) -> Self {
        let native_note_id_and_note_args: Vec<(NativeNoteId, Option<NativeNoteArgs>)> =
            notes.into();
        self.0 = self.0.clone().with_authenticated_input_notes(native_note_id_and_note_args);
        self
    }

    pub fn with_own_output_notes(mut self, notes: &OutputNotesArray) -> Self {
        let native_output_notes: Vec<NativeOutputNote> = notes.into();
        self.0 = self.0.clone().with_own_output_notes(native_output_notes).unwrap();
        self
    }

    pub fn with_custom_script(mut self, script: &TransactionScript) -> Self {
        let native_script: NativeTransactionScript = script.into();
        self.0 = self.0.clone().with_custom_script(native_script).unwrap();
        self
    }

    pub fn with_expected_output_notes(mut self, notes: &NotesArray) -> Self {
        let native_notes: Vec<NativeNote> = notes.into();
        self.0 = self.0.clone().with_expected_output_notes(native_notes);
        self
    }

    pub fn with_expected_future_notes(mut self, note_details: &NoteDetailsArray) -> Self {
        let native_note_details: Vec<NativeNoteDetails> = note_details.into();
        self.0 = self.0.clone().with_expected_future_notes(native_note_details);
        self
    }

    pub fn extend_advice_map(mut self, advice_map: &AdviceMap) -> Self {
        let native_advice_map: NativeAdviceMap = advice_map.into();
        self.0 = self.0.clone().extend_advice_map(native_advice_map);
        self
    }
}

// CONVERSIONS
// ================================================================================================

impl From<TransactionRequest> for NativeTransactionRequest {
    fn from(transaction_request: TransactionRequest) -> Self {
        transaction_request.0
    }
}

impl From<&TransactionRequest> for NativeTransactionRequest {
    fn from(transaction_request: &TransactionRequest) -> Self {
        transaction_request.0.clone()
    }
}

impl Default for TransactionRequest {
    fn default() -> Self {
        Self::new()
    }
}

use miden_client::transaction::{
    NoteArgs as NativeNoteArgs, TransactionRequest as NativeTransactionRequest,
    TransactionRequestBuilder as NativeTransactionRequestBuilder,
};
use miden_objects::{
    note::{
        Note as NativeNote, NoteDetails as NativeNoteDetails, NoteId as NativeNoteId,
        NoteTag as NativeNoteTag,
    },
    transaction::{OutputNote as NativeOutputNote, TransactionScript as NativeTransactionScript},
    vm::AdviceMap as NativeAdviceMap,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::Uint8Array;

use crate::{
    models::{
        advice_map::AdviceMap,
        note::{Note, NotesArray},
        note_details::NoteDetails,
        note_id::NoteId,
        note_tag::NoteTag,
        output_note::OutputNotesArray,
        transaction_script::TransactionScript,
        word::Word,
    },
    utils::*,
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

// HELPER STRUCTS
// ================================================================================================

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
        note_details_and_tag_array
            .0
            .into_iter()
            .map(|note_details_and_tag| note_details_and_tag.into())
            .collect()
    }
}

impl From<&NoteDetailsAndTagArray> for Vec<(NativeNoteDetails, NativeNoteTag)> {
    fn from(note_details_and_tag_array: &NoteDetailsAndTagArray) -> Self {
        note_details_and_tag_array
            .0
            .iter()
            .map(|note_details_and_tag| note_details_and_tag.into())
            .collect()
    }
}

// Transaction Request Builder
#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionRequestBuilder(NativeTransactionRequestBuilder);

// Transaction Request
#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionRequest(NativeTransactionRequest);

#[wasm_bindgen]
impl TransactionRequest {
    pub fn serialize(&self) -> Uint8Array {
        serialize_to_uint8array(&self.0)
    }

    pub fn deserialize(bytes: Uint8Array) -> Result<TransactionRequest, JsValue> {
        deserialize_from_uint8array::<NativeTransactionRequest>(bytes).map(TransactionRequest)
    }
}

#[wasm_bindgen]
impl TransactionRequestBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> TransactionRequestBuilder {
        let native_transaction_request = NativeTransactionRequestBuilder::new();
        TransactionRequestBuilder(native_transaction_request)
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

    pub fn with_expected_future_notes(
        mut self,
        note_details_and_tag: &NoteDetailsAndTagArray,
    ) -> Self {
        let native_note_details_and_tag: Vec<(NativeNoteDetails, NativeNoteTag)> =
            note_details_and_tag.into();
        self.0 = self.0.clone().with_expected_future_notes(native_note_details_and_tag);
        self
    }

    pub fn extend_advice_map(mut self, advice_map: &AdviceMap) -> Self {
        let native_advice_map: NativeAdviceMap = advice_map.into();
        self.0 = self.0.clone().extend_advice_map(native_advice_map);
        self
    }

    pub fn build(self) -> TransactionRequest {
        TransactionRequest(self.0.build())
    }
}

// CONVERSIONS
// ================================================================================================

impl From<TransactionRequestBuilder> for NativeTransactionRequestBuilder {
    fn from(transaction_request: TransactionRequestBuilder) -> Self {
        transaction_request.0
    }
}

impl From<&TransactionRequestBuilder> for NativeTransactionRequestBuilder {
    fn from(transaction_request: &TransactionRequestBuilder) -> Self {
        transaction_request.0.clone()
    }
}

impl Default for TransactionRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

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

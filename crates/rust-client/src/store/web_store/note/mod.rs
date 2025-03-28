use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use js_sys::{Array, Promise};
use miden_objects::{Digest, note::Nullifier};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{JsFuture, js_sys, wasm_bindgen};

use super::WebStore;
use crate::store::{
    InputNoteRecord, InputNoteState, NoteFilter, OutputNoteRecord, OutputNoteState, StoreError,
};

mod js_bindings;
use js_bindings::{
    idxdb_get_input_notes, idxdb_get_input_notes_from_ids, idxdb_get_input_notes_from_nullifiers,
    idxdb_get_output_notes, idxdb_get_output_notes_from_ids,
    idxdb_get_output_notes_from_nullifiers, idxdb_get_unspent_input_note_nullifiers,
};

mod models;
use models::{InputNoteIdxdbObject, OutputNoteIdxdbObject};

pub(crate) mod utils;
use utils::{parse_input_note_idxdb_object, parse_output_note_idxdb_object, upsert_input_note_tx};

impl WebStore {
    pub(crate) async fn get_input_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        let js_value =
            JsFuture::from(filter.to_input_notes_promise()).await.map_err(|js_error| {
                StoreError::DatabaseError(format!("failed to get input notes: {js_error:?}"))
            })?;
        let input_notes_idxdb: Vec<InputNoteIdxdbObject> = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        input_notes_idxdb
            .into_iter()
            .map(parse_input_note_idxdb_object) // Simplified closure
            .collect::<Result<Vec<_>, _>>() // Collect results into a single Result
    }

    pub(crate) async fn get_output_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        let js_value =
            JsFuture::from(filter.to_output_note_promise()).await.map_err(|js_error| {
                StoreError::DatabaseError(format!("failed to get output notes: {js_error:?}"))
            })?;

        let output_notes_idxdb: Vec<OutputNoteIdxdbObject> = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        output_notes_idxdb
            .into_iter()
            .map(parse_output_note_idxdb_object) // Simplified closure
            .collect::<Result<Vec<_>, _>>() // Collect results into a single Result
    }

    pub(crate) async fn get_unspent_input_note_nullifiers(
        &self,
    ) -> Result<Vec<Nullifier>, StoreError> {
        let promise = idxdb_get_unspent_input_note_nullifiers();
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!(
                "failed to get unspent input note nullifiers: {js_error:?}"
            ))
        })?;
        let nullifiers_as_str: Vec<String> = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        nullifiers_as_str
            .into_iter()
            .map(|s| Digest::try_from(s).map(Nullifier::from).map_err(StoreError::HexParseError))
            .collect::<Result<Vec<Nullifier>, _>>()
    }

    pub(crate) async fn upsert_input_notes(
        &self,
        notes: &[InputNoteRecord],
    ) -> Result<(), StoreError> {
        for note in notes {
            upsert_input_note_tx(note).await?;
        }

        Ok(())
    }
}

impl NoteFilter {
    fn to_input_notes_promise(&self) -> Promise {
        match self {
            NoteFilter::All
            | NoteFilter::Consumed
            | NoteFilter::Committed
            | NoteFilter::Expected
            | NoteFilter::Processing
            | NoteFilter::Unspent
            | NoteFilter::Unverified => {
                let states: Vec<u8> = match self {
                    NoteFilter::All => vec![],
                    NoteFilter::Consumed => vec![
                        InputNoteState::STATE_CONSUMED_AUTHENTICATED_LOCAL,
                        InputNoteState::STATE_CONSUMED_UNAUTHENTICATED_LOCAL,
                        InputNoteState::STATE_CONSUMED_EXTERNAL,
                    ],
                    NoteFilter::Committed => vec![InputNoteState::STATE_COMMITTED],
                    NoteFilter::Expected => vec![InputNoteState::STATE_EXPECTED],
                    NoteFilter::Processing => {
                        vec![
                            InputNoteState::STATE_PROCESSING_AUTHENTICATED,
                            InputNoteState::STATE_PROCESSING_UNAUTHENTICATED,
                        ]
                    },
                    NoteFilter::Unverified => vec![InputNoteState::STATE_UNVERIFIED],
                    NoteFilter::Unspent => vec![
                        InputNoteState::STATE_EXPECTED,
                        InputNoteState::STATE_COMMITTED,
                        InputNoteState::STATE_UNVERIFIED,
                        InputNoteState::STATE_PROCESSING_AUTHENTICATED,
                        InputNoteState::STATE_PROCESSING_UNAUTHENTICATED,
                    ],
                    _ => unreachable!(), // Safety net, should never be reached
                };

                // Assuming `js_fetch_notes` is your JavaScript function that handles simple string
                // filters
                idxdb_get_input_notes(states)
            },
            NoteFilter::List(ids) => {
                let note_ids_as_str: Vec<String> =
                    ids.iter().map(|id| id.inner().to_string()).collect();
                idxdb_get_input_notes_from_ids(note_ids_as_str)
            },
            NoteFilter::Unique(id) => {
                let note_id_as_str = id.inner().to_string();
                let note_ids = vec![note_id_as_str];
                idxdb_get_input_notes_from_ids(note_ids)
            },
            NoteFilter::Nullifiers(nullifiers) => {
                let nullifiers_as_str =
                    nullifiers.iter().map(ToString::to_string).collect::<Vec<String>>();

                idxdb_get_input_notes_from_nullifiers(nullifiers_as_str)
            },
        }
    }

    fn to_output_note_promise(&self) -> Promise {
        match self {
            NoteFilter::All
            | NoteFilter::Consumed
            | NoteFilter::Committed
            | NoteFilter::Expected
            | NoteFilter::Unspent => {
                let states = match self {
                    NoteFilter::All => vec![],
                    NoteFilter::Consumed => vec![OutputNoteState::STATE_CONSUMED],
                    NoteFilter::Committed => vec![
                        OutputNoteState::STATE_COMMITTED_FULL,
                        OutputNoteState::STATE_COMMITTED_PARTIAL,
                    ],
                    NoteFilter::Expected => vec![
                        OutputNoteState::STATE_EXPECTED_FULL,
                        OutputNoteState::STATE_EXPECTED_PARTIAL,
                    ],
                    NoteFilter::Unspent => vec![
                        OutputNoteState::STATE_EXPECTED_FULL,
                        OutputNoteState::STATE_COMMITTED_FULL,
                    ],
                    _ => unreachable!(), // Safety net, should never be reached
                };

                idxdb_get_output_notes(states)
            },
            NoteFilter::Processing | NoteFilter::Unverified => {
                Promise::resolve(&JsValue::from(Array::new()))
            },
            NoteFilter::List(ids) => {
                let note_ids_as_str: Vec<String> =
                    ids.iter().map(|id| id.inner().to_string()).collect();
                idxdb_get_output_notes_from_ids(note_ids_as_str)
            },
            NoteFilter::Unique(id) => {
                let note_id_as_str = id.inner().to_string();
                let note_ids = vec![note_id_as_str];
                idxdb_get_output_notes_from_ids(note_ids)
            },
            NoteFilter::Nullifiers(nullifiers) => {
                let nullifiers_as_str =
                    nullifiers.iter().map(ToString::to_string).collect::<Vec<String>>();

                idxdb_get_output_notes_from_nullifiers(nullifiers_as_str)
            },
        }
    }
}

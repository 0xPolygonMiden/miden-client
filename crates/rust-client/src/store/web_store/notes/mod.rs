use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use js_sys::{Array, Promise};
use miden_objects::{notes::Nullifier, Digest};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::*;

use super::WebStore;
use crate::store::{
    note_record::{
        STATE_COMMITTED, STATE_COMMITTED_FULL, STATE_COMMITTED_PARTIAL, STATE_CONSUMED,
        STATE_CONSUMED_AUTHENTICATED_LOCAL, STATE_CONSUMED_EXTERNAL,
        STATE_CONSUMED_UNAUTHENTICATED_LOCAL, STATE_EXPECTED, STATE_EXPECTED_FULL,
        STATE_EXPECTED_PARTIAL, STATE_PROCESSING_AUTHENTICATED, STATE_PROCESSING_UNAUTHENTICATED,
    },
    InputNoteRecord, NoteFilter, OutputNoteRecord, StoreError,
};

mod js_bindings;
use js_bindings::*;

mod models;
use models::*;

pub(crate) mod utils;
use utils::*;

impl WebStore {
    pub(crate) async fn get_input_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        let js_value = JsFuture::from(filter.to_input_notes_promise()).await.unwrap();
        let input_notes_idxdb: Vec<InputNoteIdxdbObject> = from_value(js_value).unwrap();

        let native_input_notes: Result<Vec<InputNoteRecord>, StoreError> = input_notes_idxdb
            .into_iter()
            .map(parse_input_note_idxdb_object) // Simplified closure
            .collect::<Result<Vec<_>, _>>(); // Collect results into a single Result

        match native_input_notes {
            Ok(ref notes) => match &filter {
                NoteFilter::Unique(note_id) if notes.is_empty() => {
                    return Err(StoreError::NoteNotFound(*note_id));
                },
                NoteFilter::List(note_ids) if note_ids.len() != notes.len() => {
                    let missing_note_id = note_ids
                        .iter()
                        .find(|&&note_id| {
                            !notes.iter().any(|note_record| note_record.id() == note_id)
                        })
                        .expect("should find one note id that wasn't retrieved by the db");
                    return Err(StoreError::NoteNotFound(*missing_note_id));
                },
                _ => {},
            },
            Err(e) => return Err(e),
        }

        native_input_notes
    }

    pub(crate) async fn get_output_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        let js_value = JsFuture::from(filter.to_output_note_promise()).await.unwrap();

        let output_notes_idxdb: Vec<OutputNoteIdxdbObject> = from_value(js_value).unwrap();

        let native_output_notes: Result<Vec<OutputNoteRecord>, StoreError> = output_notes_idxdb
            .into_iter()
            .map(parse_output_note_idxdb_object) // Simplified closure
            .collect::<Result<Vec<_>, _>>(); // Collect results into a single Result

        match native_output_notes {
            Ok(ref notes) => match filter {
                NoteFilter::Unique(note_id) if notes.is_empty() => {
                    return Err(StoreError::NoteNotFound(note_id));
                },
                NoteFilter::List(note_ids) if note_ids.len() != notes.len() => {
                    let missing_note_id = note_ids
                        .iter()
                        .find(|&&note_id| {
                            !notes.iter().any(|note_record| note_record.id() == note_id)
                        })
                        .expect("should find one note id that wasn't retrieved by the db");
                    return Err(StoreError::NoteNotFound(*missing_note_id));
                },
                _ => {},
            },
            Err(e) => return Err(e),
        }

        native_output_notes
    }

    pub(crate) async fn get_unspent_input_note_nullifiers(
        &self,
    ) -> Result<Vec<Nullifier>, StoreError> {
        let promise = idxdb_get_unspent_input_note_nullifiers();
        let js_value = JsFuture::from(promise).await.unwrap();
        let nullifiers_as_str: Vec<String> = from_value(js_value).unwrap();

        nullifiers_as_str
            .into_iter()
            .map(|s| Digest::try_from(s).map(Nullifier::from).map_err(StoreError::HexParseError))
            .collect::<Result<Vec<Nullifier>, _>>()
    }

    pub(crate) async fn upsert_input_notes(
        &self,
        notes: Vec<InputNoteRecord>,
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
            | NoteFilter::StateDiscriminant(_) => {
                let states: Vec<u8> = match self {
                    NoteFilter::All => vec![],
                    NoteFilter::Consumed => vec![
                        STATE_CONSUMED_AUTHENTICATED_LOCAL,
                        STATE_CONSUMED_UNAUTHENTICATED_LOCAL,
                        STATE_CONSUMED_EXTERNAL,
                    ],
                    NoteFilter::Committed => vec![STATE_COMMITTED],
                    NoteFilter::Expected => vec![STATE_EXPECTED],
                    NoteFilter::Processing => {
                        vec![STATE_PROCESSING_AUTHENTICATED, STATE_PROCESSING_UNAUTHENTICATED]
                    },
                    NoteFilter::StateDiscriminant(discriminant) => vec![*discriminant],
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
                let nullifiers_as_str = nullifiers
                    .iter()
                    .map(|nullifier| nullifier.to_string())
                    .collect::<Vec<String>>();

                idxdb_get_input_notes_from_nullifiers(nullifiers_as_str)
            },
            NoteFilter::Or(filters) => {
                let mut promises: Vec<JsValue> = Vec::new();
                for filter in filters {
                    promises.push(filter.to_input_notes_promise().into());
                }

                Promise::all(&Array::from_iter(promises.iter()))
            },
        }
    }

    fn to_output_note_promise(&self) -> Promise {
        match self {
            NoteFilter::All
            | NoteFilter::Consumed
            | NoteFilter::Committed
            | NoteFilter::Expected => {
                let states = match self {
                    NoteFilter::All => vec![],
                    NoteFilter::Consumed => vec![STATE_CONSUMED],
                    NoteFilter::Committed => vec![STATE_COMMITTED_FULL, STATE_COMMITTED_PARTIAL],
                    NoteFilter::Expected => vec![STATE_EXPECTED_FULL, STATE_EXPECTED_PARTIAL],
                    _ => unreachable!(), // Safety net, should never be reached
                };

                idxdb_get_output_notes(states)
            },
            NoteFilter::Processing => Promise::resolve(&JsValue::from(Array::new())),
            NoteFilter::List(ref ids) => {
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
                let nullifiers_as_str = nullifiers
                    .iter()
                    .map(|nullifier| nullifier.to_string())
                    .collect::<Vec<String>>();

                idxdb_get_output_notes_from_nullifiers(nullifiers_as_str)
            },
            NoteFilter::StateDiscriminant(discriminant) => {
                idxdb_get_output_notes(vec![*discriminant])
            },
            NoteFilter::Or(filters) => {
                let mut promises: Vec<JsValue> = Vec::new();
                for filter in filters {
                    promises.push(filter.to_output_note_promise().into());
                }

                Promise::all(&Array::from_iter(promises.iter()))
            },
        }
    }
}

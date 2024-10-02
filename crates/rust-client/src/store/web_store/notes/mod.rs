use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    notes::{NoteId, NoteInclusionProof, NoteMetadata, Nullifier},
    Digest,
};
use miden_tx::utils::Serializable;
use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::*;

use super::WebStore;
use crate::store::{InputNoteRecord, NoteFilter, OutputNoteRecord, StoreError};

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
        let promise = match filter {
            NoteFilter::All
            | NoteFilter::Consumed
            | NoteFilter::Committed
            | NoteFilter::Expected
            | NoteFilter::Processing => {
                let filter_as_str = match filter {
                    NoteFilter::All => "All",
                    NoteFilter::Consumed => "Consumed",
                    NoteFilter::Committed => "Committed",
                    NoteFilter::Expected => "Expected",
                    NoteFilter::Processing => "Processing",
                    _ => unreachable!(), // Safety net, should never be reached
                };

                // Assuming `js_fetch_notes` is your JavaScript function that handles simple string
                // filters
                idxdb_get_input_notes(filter_as_str.to_string())
            },
            NoteFilter::Ignored => idxdb_get_ignored_input_notes(),
            NoteFilter::List(ref ids) => {
                let note_ids_as_str: Vec<String> =
                    ids.iter().map(|id| id.inner().to_string()).collect();
                idxdb_get_input_notes_from_ids(note_ids_as_str)
            },
            NoteFilter::Unique(id) => {
                let note_id_as_str = id.inner().to_string();
                let note_ids = vec![note_id_as_str];
                idxdb_get_input_notes_from_ids(note_ids)
            },
        };

        let js_value = JsFuture::from(promise).await.unwrap();
        let input_notes_idxdb: Vec<InputNoteIdxdbObject> = from_value(js_value).unwrap();

        let native_input_notes: Result<Vec<InputNoteRecord>, StoreError> = input_notes_idxdb
            .into_iter()
            .map(parse_input_note_idxdb_object) // Simplified closure
            .collect::<Result<Vec<_>, _>>(); // Collect results into a single Result

        match native_input_notes {
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

        native_input_notes
    }

    pub(crate) async fn get_output_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        let promise = match filter {
            NoteFilter::All
            | NoteFilter::Consumed
            | NoteFilter::Committed
            | NoteFilter::Expected
            | NoteFilter::Processing => {
                let filter_as_str = match filter {
                    NoteFilter::All => "All",
                    NoteFilter::Consumed => "Consumed",
                    NoteFilter::Committed => "Committed",
                    NoteFilter::Expected => "Expected",
                    NoteFilter::Processing => "Processing",
                    _ => unreachable!(), // Safety net, should never be reached
                };

                // Assuming `js_fetch_notes` is your JavaScript function that handles simple string
                // filters

                idxdb_get_output_notes(filter_as_str.to_string())
            },
            NoteFilter::Ignored => idxdb_get_ignored_output_notes(),
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
        };

        let js_value = JsFuture::from(promise).await.unwrap();

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

    pub(crate) async fn insert_input_note(&self, note: InputNoteRecord) -> Result<(), StoreError> {
        let block_num = self.get_sync_height().await?;

        insert_input_note_tx(block_num, note).await
    }

    pub async fn update_note_inclusion_proof(
        &self,
        note_id: NoteId,
        inclusion_proof: NoteInclusionProof,
    ) -> Result<(), StoreError> {
        let note_id_as_str = note_id.inner().to_string();
        let inclusion_proof_as_vec = inclusion_proof.to_bytes();

        let promise = idxdb_update_note_inclusion_proof(note_id_as_str, inclusion_proof_as_vec);
        let _ = JsFuture::from(promise).await.unwrap();

        Ok(())
    }

    pub async fn update_note_metadata(
        &self,
        note_id: NoteId,
        metadata: NoteMetadata,
    ) -> Result<(), StoreError> {
        let note_id_as_str = note_id.inner().to_string();
        let metadata_as_vec = metadata.to_bytes();

        let promise = idxdb_update_note_metadata(note_id_as_str, metadata_as_vec);
        let _ = JsFuture::from(promise).await.unwrap();

        Ok(())
    }
}

use miden_objects::notes::{NoteAssets, NoteId, NoteInclusionProof, NoteInputs, NoteMetadata, NoteScript, Nullifier};
use miden_objects::Digest;
use miden_tx::utils::Deserializable;
use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::*;

use super::WebStore;
use crate::native_code::errors::StoreError;
use crate::native_code::store::note_record::{
    InputNoteRecord, 
    NoteRecordDetails, 
    NoteStatus, 
    OutputNoteRecord
};
use crate::native_code::store::NoteFilter;
use crate::web_client::notes::WebClientNoteFilter;

mod js_bindings;
use js_bindings::*;

mod models;
use models::*;

pub(crate) mod utils;
use utils::*;

impl WebStore {
    pub(crate) async fn get_input_notes(
        &self,
        filter: NoteFilter
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        let filter_as_str = match filter {
            NoteFilter::Pending => "Pending",
            NoteFilter::Committed => "Committed",
            NoteFilter::Consumed => "Consumed",
            NoteFilter::All => "All", 
        };

        let promise = idxdb_get_input_notes(filter_as_str.to_string());
        let js_value = JsFuture::from(promise).await.unwrap();
        let input_notes_idxdb: Vec<InputNoteIdxdbObject> = from_value(js_value).unwrap();
  
        let native_input_notes: Result<Vec<InputNoteRecord>, StoreError> = input_notes_idxdb.into_iter().map(|note_idxdb| {

            // Merge the info that comes from the input notes table and the notes script table
            let note_script = NoteScript::read_from_bytes(&note_idxdb.serialized_note_script)?;
            let note_details: NoteRecordDetails =
                serde_json::from_str(&note_idxdb.details).map_err(StoreError::JsonDataDeserializationError)?;
            let note_details = NoteRecordDetails::new(
                note_details.nullifier().to_string(),
                note_script,
                note_details.inputs().clone(),
                note_details.serial_num(),
            );

            let note_metadata: Option<NoteMetadata> = if let Some(metadata_as_json_str) = note_idxdb.metadata {
                Some(
                    serde_json::from_str(&metadata_as_json_str)
                        .map_err(StoreError::JsonDataDeserializationError)?,
                )
            } else {
                None
            };

            let note_assets = NoteAssets::read_from_bytes(&note_idxdb.assets)?;

            let inclusion_proof = match note_idxdb.inclusion_proof {
                Some(note_inclusion_proof) => {
                    let note_inclusion_proof: NoteInclusionProof =
                        serde_json::from_str(&note_inclusion_proof)
                            .map_err(StoreError::JsonDataDeserializationError)?;

                    Some(note_inclusion_proof)
                },
                _ => None,
            };

            let recipient = Digest::try_from(note_idxdb.recipient)?;
            let id = NoteId::new(recipient, note_assets.commitment());
            let status: NoteStatus = serde_json::from_str(&format!("\"{0}\"", note_idxdb.status))
                .map_err(StoreError::JsonDataDeserializationError)?;

            Ok(InputNoteRecord::new(
                id,
                recipient,
                note_assets,
                status,
                note_metadata,
                inclusion_proof,
                note_details,
            ))
        }).collect();
        
        return native_input_notes;
    }

    pub(crate) async fn get_input_note(
        &self,
        note_id: NoteId
    ) -> Result<InputNoteRecord, StoreError> {
        let note_id_str = &note_id.inner().to_string();

        let promise = idxdb_get_input_note(note_id_str.to_string());
        let js_value = JsFuture::from(promise).await.unwrap();
        let input_note_idxdb: InputNoteIdxdbObject = from_value(js_value).unwrap();

        // Merge the info that comes from the input notes table and the notes script table
        let note_script = NoteScript::read_from_bytes(&input_note_idxdb.serialized_note_script)?;
        let note_details: NoteRecordDetails =
            serde_json::from_str(&input_note_idxdb.details).map_err(StoreError::JsonDataDeserializationError)?;
        let note_details = NoteRecordDetails::new(
            note_details.nullifier().to_string(),
            note_script,
            note_details.inputs().clone(),
            note_details.serial_num(),
        );

        let note_metadata: Option<NoteMetadata> = if let Some(metadata_as_json_str) = input_note_idxdb.metadata {
            Some(
                serde_json::from_str(&metadata_as_json_str)
                    .map_err(StoreError::JsonDataDeserializationError)?,
            )
        } else {
            None
        };

        let note_assets = NoteAssets::read_from_bytes(&input_note_idxdb.assets)?;

        let inclusion_proof = match input_note_idxdb.inclusion_proof {
            Some(note_inclusion_proof) => {
                let note_inclusion_proof: NoteInclusionProof =
                    serde_json::from_str(&note_inclusion_proof)
                        .map_err(StoreError::JsonDataDeserializationError)?;

                Some(note_inclusion_proof)
            },
            _ => None,
        };

        let recipient = Digest::try_from(input_note_idxdb.recipient)?;
        let id = NoteId::new(recipient, note_assets.commitment());
        let status: NoteStatus = serde_json::from_str(&format!("\"{0}\"", input_note_idxdb.status))
            .map_err(StoreError::JsonDataDeserializationError)?;

        Ok(InputNoteRecord::new(
            id,
            recipient,
            note_assets,
            status,
            note_metadata,
            inclusion_proof,
            note_details,
        ))
    }

    pub(crate) async fn insert_input_note(
        &mut self,
        note: &InputNoteRecord
    ) -> Result<(), StoreError> {
        insert_input_note_tx(note).await
    }

    pub(crate) async fn get_output_notes(
        &self,
        filter: NoteFilter
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        let filter_as_str = match filter {
            NoteFilter::Pending => "Pending",
            NoteFilter::Committed => "Committed",
            NoteFilter::Consumed => "Consumed",
            NoteFilter::All => "All", 
        };

        let promise = idxdb_get_output_notes(filter_as_str.to_string());
        let js_value = JsFuture::from(promise).await.unwrap();
        let output_notes_idxdb: Vec<OutputNoteIdxdbObject> = from_value(js_value).unwrap();
  
        let native_output_notes: Result<Vec<OutputNoteRecord>, StoreError> = output_notes_idxdb.into_iter().map(|note_idxdb| {
            let note_details: Option<NoteRecordDetails> = if let Some(details_as_json_str) = note_idxdb.details {
                // Merge the info that comes from the input notes table and the notes script table
                let serialized_note_script = note_idxdb.serialized_note_script
                    .expect("Has note details so it should have the serialized script");
                let note_script = NoteScript::read_from_bytes(&serialized_note_script)?;
                let note_details: NoteRecordDetails = serde_json::from_str(&details_as_json_str)
                    .map_err(StoreError::JsonDataDeserializationError)?;
                let note_details = NoteRecordDetails::new(
                    note_details.nullifier().to_string(),
                    note_script,
                    note_details.inputs().clone(),
                    note_details.serial_num(),
                );
        
                Some(note_details)
            } else {
                None
            };
        
            let note_metadata: NoteMetadata =
                serde_json::from_str(&note_idxdb.metadata).map_err(StoreError::JsonDataDeserializationError)?;
            let note_assets = NoteAssets::read_from_bytes(&note_idxdb.assets)?;
        
            let inclusion_proof = match note_idxdb.inclusion_proof {
                Some(note_inclusion_proof) => {
                    let note_inclusion_proof: NoteInclusionProof =
                        serde_json::from_str(&note_inclusion_proof)
                            .map_err(StoreError::JsonDataDeserializationError)?;
        
                    Some(note_inclusion_proof)
                },
                _ => None,
            };
        
            let recipient = Digest::try_from(note_idxdb.recipient)?;
            let id = NoteId::new(recipient, note_assets.commitment());
            let status: NoteStatus = serde_json::from_str(&format!("\"{0}\"", note_idxdb.status))
                .map_err(StoreError::JsonDataDeserializationError)?;
        
            Ok(OutputNoteRecord::new(
                id,
                recipient,
                note_assets,
                status,
                note_metadata,
                inclusion_proof,
                note_details,
            ))
        }).collect(); // Collect into a Result<Vec<AccountId>, ()>
        
        return native_output_notes;
    }

    pub(crate) async fn get_unspent_input_note_nullifiers(
        &self
    ) -> Result<Vec<Nullifier>, StoreError>{
        let promise = idxdb_get_unspent_input_note_nullifiers();
        let js_value = JsFuture::from(promise).await.unwrap();
        let nullifiers_as_str: Vec<String> = from_value(js_value).unwrap();

        let nullifiers = nullifiers_as_str.into_iter().map(|s| {
            Digest::try_from(s).map(Nullifier::from).map_err(StoreError::HexParseError)
        }).collect::<Result<Vec<Nullifier>, _>>();

        return nullifiers;
    }
}
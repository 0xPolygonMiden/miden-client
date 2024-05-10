use miden_objects::notes::NoteInclusionProof;
use miden_tx::utils::Serializable;
use wasm_bindgen_futures::*;

use crate::native_code::{errors::StoreError, store::note_record::{InputNoteRecord, NoteStatus, OutputNoteRecord}};

use super::js_bindings::*;

// TYPES
// ================================================================================================

type SerializedInputNoteData = (
    String, 
    Vec<u8>, 
    String, 
    String, 
    Option<String>, 
    String, 
    String,
    Vec<u8>,
    Option<String>
);

type SerializedOutputNoteData = (
    String,
    Vec<u8>,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<Vec<u8>>,
    Option<String>,
);

// ================================================================================================

pub(crate) fn serialize_input_note(
    note: &InputNoteRecord
) -> Result<SerializedInputNoteData, StoreError> {
    let note_id = note.id().inner().to_string();
    let note_assets = note.assets().to_bytes();

    let (inclusion_proof, status) = match note.inclusion_proof() {
        Some(proof) => {
            let block_num = proof.origin().block_num;
            let node_index = proof.origin().node_index.value();
            let sub_hash = proof.sub_hash();
            let note_root = proof.note_root();

            let inclusion_proof = serde_json::to_string(&NoteInclusionProof::new(
                block_num,
                sub_hash,
                note_root,
                node_index,
                proof.note_path().clone(),
            )?)
            .map_err(StoreError::InputSerializationError)?;

            let status = serde_json::to_string(&NoteStatus::Committed)
                .map_err(StoreError::InputSerializationError)?
                .replace('\"', "");
            (Some(inclusion_proof), status)
        },
        None => {
            let status = serde_json::to_string(&NoteStatus::Pending)
                .map_err(StoreError::InputSerializationError)?
                .replace('\"', "");

            (None, status)
        },
    };
    let recipient = note.recipient().to_hex();

    let metadata = if let Some(metadata) = note.metadata() {
        Some(serde_json::to_string(metadata).map_err(StoreError::InputSerializationError)?)
    } else {
        None
    };

    let details =
        serde_json::to_string(&note.details()).map_err(StoreError::InputSerializationError)?;
    let note_script_hash = note.details().script_hash().to_hex();
    let serialized_note_script = note.details().script().to_bytes();

    Ok((
        note_id,
        note_assets,
        recipient,
        status,
        metadata,
        details,
        note_script_hash,
        serialized_note_script,
        inclusion_proof,
    ))
}

pub async fn insert_input_note_tx(
    note: &InputNoteRecord
) -> Result<(), StoreError> {
    let (
        note_id, 
        assets, 
        recipient, 
        status, 
        metadata, 
        details, 
        note_script_hash,
        serialized_note_script,
        inclusion_proof
    ) = serialize_input_note(note)?;

    let promise = idxdb_insert_input_note(
        note_id,
        assets,
        recipient,
        status,
        metadata,
        details,
        note_script_hash,
        serialized_note_script,
        inclusion_proof
    );
    JsFuture::from(promise).await.unwrap();

    Ok(())
}

pub(crate) fn serialize_output_note(
    note: &OutputNoteRecord,
) -> Result<SerializedOutputNoteData, StoreError> {
    let note_id = note.id().inner().to_string();
    let note_assets = note.assets().to_bytes();
    let (inclusion_proof, status) = match note.inclusion_proof() {
        Some(proof) => {
            let block_num = proof.origin().block_num;
            let node_index = proof.origin().node_index.value();
            let sub_hash = proof.sub_hash();
            let note_root = proof.note_root();

            let inclusion_proof = serde_json::to_string(&NoteInclusionProof::new(
                block_num,
                sub_hash,
                note_root,
                node_index,
                proof.note_path().clone(),
            )?)
            .map_err(StoreError::InputSerializationError)?;

            let status = serde_json::to_string(&NoteStatus::Committed)
                .map_err(StoreError::InputSerializationError)?
                .replace('\"', "");

            (Some(inclusion_proof), status)
        },
        None => {
            let status = serde_json::to_string(&NoteStatus::Pending)
                .map_err(StoreError::InputSerializationError)?
                .replace('\"', "");

            (None, status)
        },
    };
    let recipient = note.recipient().to_hex();

    let metadata =
        serde_json::to_string(note.metadata()).map_err(StoreError::InputSerializationError)?;

    let details = if let Some(details) = note.details() {
        Some(serde_json::to_string(&details).map_err(StoreError::InputSerializationError)?)
    } else {
        None
    };
    let note_script_hash = note.details().map(|details| details.script_hash().to_hex());
    let serialized_note_script = note.details().map(|details| details.script().to_bytes());

    Ok((
        note_id,
        note_assets,
        recipient,
        status,
        metadata,
        details,
        note_script_hash,
        serialized_note_script,
        inclusion_proof,
    ))
}

pub async fn insert_output_note_tx(
    note: &OutputNoteRecord
) -> Result<(), StoreError> {
    let (
        note_id, 
        assets, 
        recipient,
        status, 
        metadata, 
        details,
        note_script_hash,
        serialized_note_script, 
        inclusion_proof
    ) = serialize_output_note(note)?;

    let result = JsFuture::from(idxdb_insert_output_note(
        note_id,
        assets,
        recipient,
        status,
        metadata,
        details,
        note_script_hash,
        serialized_note_script,
        inclusion_proof
    )).await; 
    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(StoreError::QueryError("Failed to insert output note".to_string())),
    }
}
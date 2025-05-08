use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use chrono::Utc;
use miden_objects::{
    Digest, Word,
    note::{NoteAssets, NoteDetails, NoteInputs, NoteMetadata, NoteRecipient, NoteScript},
    utils::Deserializable,
};
use miden_tx::utils::Serializable;
use wasm_bindgen_futures::JsFuture;

use super::{
    InputNoteIdxdbObject, OutputNoteIdxdbObject,
    js_bindings::{idxdb_upsert_input_note, idxdb_upsert_output_note},
};
use crate::{
    note::NoteUpdateTracker,
    store::{InputNoteRecord, InputNoteState, OutputNoteRecord, OutputNoteState, StoreError},
};

// TYPES
// ================================================================================================

pub struct SerializedInputNoteData {
    pub note_id: String,
    pub note_assets: Vec<u8>,
    pub serial_number: Vec<u8>,
    pub inputs: Vec<u8>,
    pub note_script_root: String,
    pub note_script: Vec<u8>,
    pub nullifier: String,
    pub state_discriminant: u8,
    pub state: Vec<u8>,
    pub created_at: String,
}

pub struct SerializedOutputNoteData {
    pub note_id: String,
    pub note_assets: Vec<u8>,
    pub recipient_digest: String,
    pub metadata: Vec<u8>,
    pub nullifier: Option<String>,
    pub expected_height: u32,
    pub state_discriminant: u8,
    pub state: Vec<u8>,
}

// ================================================================================================

pub(crate) fn serialize_input_note(note: &InputNoteRecord) -> SerializedInputNoteData {
    let note_id = note.id().inner().to_string();
    let note_assets = note.assets().to_bytes();

    let details = note.details();
    let serial_number = details.serial_num().to_bytes();
    let inputs = details.inputs().to_bytes();
    let nullifier = details.nullifier().to_hex();

    let recipient = details.recipient();
    let note_script: Vec<u8> = recipient.script().to_bytes();
    let note_script_root = recipient.script().root().to_hex();

    let state_discriminant = note.state().discriminant();
    let state = note.state().to_bytes();
    let created_at = Utc::now().timestamp().to_string();

    SerializedInputNoteData {
        note_id,
        note_assets,
        serial_number,
        inputs,
        note_script_root,
        note_script,
        nullifier,
        state_discriminant,
        state,
        created_at,
    }
}

pub async fn upsert_input_note_tx(note: &InputNoteRecord) -> Result<(), StoreError> {
    let serialized_data = serialize_input_note(note);

    let promise = idxdb_upsert_input_note(
        serialized_data.note_id,
        serialized_data.note_assets,
        serialized_data.serial_number,
        serialized_data.inputs,
        serialized_data.note_script_root,
        serialized_data.note_script,
        serialized_data.nullifier,
        serialized_data.created_at,
        serialized_data.state_discriminant,
        serialized_data.state,
    );
    JsFuture::from(promise).await.map_err(|js_error| {
        StoreError::DatabaseError(format!("failed to upsert input note: {js_error:?}"))
    })?;

    Ok(())
}

pub(crate) fn serialize_output_note(note: &OutputNoteRecord) -> SerializedOutputNoteData {
    let note_id = note.id().inner().to_string();
    let note_assets = note.assets().to_bytes();
    let recipient_digest = note.recipient_digest().to_hex();
    let metadata = note.metadata().to_bytes();

    let nullifier = note.nullifier().map(|nullifier| nullifier.to_hex());

    let state_discriminant = note.state().discriminant();
    let state = note.state().to_bytes();

    SerializedOutputNoteData {
        note_id,
        note_assets,
        recipient_digest,
        metadata,
        nullifier,
        state_discriminant,
        state,
        expected_height: note.expected_height().as_u32(),
    }
}

pub async fn upsert_output_note_tx(note: &OutputNoteRecord) -> Result<(), StoreError> {
    let serialized_data = serialize_output_note(note);

    let result = JsFuture::from(idxdb_upsert_output_note(
        serialized_data.note_id,
        serialized_data.note_assets,
        serialized_data.recipient_digest,
        serialized_data.metadata,
        serialized_data.nullifier,
        serialized_data.expected_height,
        serialized_data.state_discriminant,
        serialized_data.state,
    ))
    .await;
    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(StoreError::QueryError("Failed to insert output note".to_string())),
    }
}

pub fn parse_input_note_idxdb_object(
    note_idxdb: InputNoteIdxdbObject,
) -> Result<InputNoteRecord, StoreError> {
    // Merge the info that comes from the input notes table and the notes script table
    let InputNoteIdxdbObject {
        assets,
        serial_number,
        inputs,
        serialized_note_script,
        state,
        created_at,
    } = note_idxdb;

    let assets = NoteAssets::read_from_bytes(&assets)?;

    let serial_number = Word::read_from_bytes(&serial_number)?;
    let script = NoteScript::read_from_bytes(&serialized_note_script)?;
    let inputs = NoteInputs::read_from_bytes(&inputs)?;
    let recipient = NoteRecipient::new(serial_number, script, inputs);

    let details = NoteDetails::new(assets, recipient);

    let state = InputNoteState::read_from_bytes(&state)?;
    let created_at = created_at
        .parse::<u64>()
        .map_err(|_| StoreError::QueryError("Failed to parse created_at timestamp".to_string()))?;

    Ok(InputNoteRecord::new(details, Some(created_at), state))
}

pub fn parse_output_note_idxdb_object(
    note_idxdb: OutputNoteIdxdbObject,
) -> Result<OutputNoteRecord, StoreError> {
    let note_metadata = NoteMetadata::read_from_bytes(&note_idxdb.metadata)?;
    let note_assets = NoteAssets::read_from_bytes(&note_idxdb.assets)?;
    let recipient = Digest::try_from(note_idxdb.recipient_digest)?;
    let state = OutputNoteState::read_from_bytes(&note_idxdb.state)?;

    Ok(OutputNoteRecord::new(
        recipient,
        note_assets,
        note_metadata,
        state,
        note_idxdb.expected_height.into(),
    ))
}

pub(crate) async fn apply_note_updates_tx(
    note_updates: &NoteUpdateTracker,
) -> Result<(), StoreError> {
    for input_note in note_updates.updated_input_notes() {
        upsert_input_note_tx(input_note.inner()).await?;
    }

    for output_note in note_updates.updated_output_notes() {
        upsert_output_note_tx(output_note.inner()).await?;
    }

    Ok(())
}

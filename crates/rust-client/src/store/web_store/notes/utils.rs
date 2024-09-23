use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use chrono::Utc;
use miden_objects::{
    accounts::AccountId,
    notes::{
        NoteAssets, NoteDetails, NoteId, NoteInclusionProof, NoteInputs, NoteMetadata,
        NoteRecipient, NoteScript,
    },
    utils::Deserializable,
    Digest, Word,
};
use miden_tx::utils::{DeserializationError, Serializable};
use wasm_bindgen_futures::*;

use super::{js_bindings::*, InputNoteIdxdbObject, OutputNoteIdxdbObject};
use crate::store::{
    note_record::{
        NOTE_STATUS_COMMITTED, NOTE_STATUS_CONSUMED, NOTE_STATUS_EXPECTED, NOTE_STATUS_PROCESSING,
    },
    InputNoteRecord, NoteRecordDetails, NoteState, NoteStatus, OutputNoteRecord, StoreError,
};

// TYPES
// ================================================================================================

pub struct SerializedInputNoteData {
    pub note_id: String,
    pub note_assets: Vec<u8>,
    pub serial_number: Vec<u8>,
    pub inputs: Vec<u8>,
    pub note_script_hash: String,
    pub note_script: Vec<u8>,
    pub nullifier: String,
    pub state_discriminant: u8,
    pub state: Vec<u8>,
    pub created_at: String,
}

pub struct SerializedOutputNoteData {
    pub note_id: String,
    pub note_assets: Vec<u8>,
    pub recipient: String,
    pub status: String,
    pub metadata: Vec<u8>,
    pub details: Option<Vec<u8>>,
    pub note_script_hash: Option<String>,
    pub note_script: Option<Vec<u8>>,
    pub inclusion_proof: Option<Vec<u8>>,
    pub created_at: String,
    pub expected_height: Option<String>,
}

// ================================================================================================

pub(crate) fn serialize_input_note(
    note: InputNoteRecord,
) -> Result<SerializedInputNoteData, StoreError> {
    let note_id = note.id().inner().to_string();
    let note_assets = note.assets().to_bytes();

    let details = note.details();
    let serial_number = details.serial_num().to_bytes();
    let inputs = details.inputs().to_bytes();
    let nullifier = details.nullifier().to_hex();

    let recipient = details.recipient();
    let note_script = recipient.script().to_bytes();
    let note_script_hash = recipient.script().hash().to_hex();

    let state_discriminant = note.state().discriminant();
    let state = note.state().to_bytes();
    let created_at = Utc::now().timestamp().to_string();

    Ok(SerializedInputNoteData {
        note_id,
        note_assets,
        serial_number,
        inputs,
        note_script_hash,
        note_script,
        nullifier,
        state_discriminant,
        state,
        created_at,
    })
}

pub async fn upsert_input_note_tx(note: InputNoteRecord) -> Result<(), StoreError> {
    let serialized_data = serialize_input_note(note)?;

    let promise = idxdb_upsert_input_note(
        serialized_data.note_id,
        serialized_data.note_assets,
        serialized_data.serial_number,
        serialized_data.inputs,
        serialized_data.note_script_hash,
        serialized_data.note_script,
        serialized_data.nullifier,
        serialized_data.created_at,
        serialized_data.state_discriminant,
        serialized_data.state,
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
            let inclusion_proof = proof.to_bytes();

            let status = NOTE_STATUS_COMMITTED.to_string();

            (Some(inclusion_proof), status)
        },
        None => {
            let status = NOTE_STATUS_EXPECTED.to_string();

            (None, status)
        },
    };
    let recipient = note.recipient().to_hex();

    let metadata = note.metadata().to_bytes();

    let details = note.details().map(|d| d.to_bytes());

    let note_script_hash = note.details().map(|details| details.script_hash().to_hex());
    let note_script = note.details().map(|details| details.script().to_bytes());
    let created_at = Utc::now().timestamp().to_string();
    let expected_height = match note.status() {
        NoteStatus::Expected { block_height, .. } => block_height.map(|height| height.to_string()),
        _ => None,
    };

    Ok(SerializedOutputNoteData {
        note_id,
        note_assets,
        recipient,
        status,
        metadata,
        details,
        note_script_hash,
        note_script,
        inclusion_proof,
        created_at,
        expected_height,
    })
}

pub async fn insert_output_note_tx(
    block_num: u32,
    note: &OutputNoteRecord,
) -> Result<(), StoreError> {
    let serialized_data = serialize_output_note(note)?;
    let expected_height = serialized_data.expected_height.or(Some(block_num.to_string()));
    let nullifier: Option<String> = match serialized_data.details {
        Some(ref bytes) => NoteRecordDetails::read_from_bytes(bytes)
            .map(|details| details.nullifier().to_string())
            .ok(),
        None => None,
    };

    let result = JsFuture::from(idxdb_insert_output_note(
        serialized_data.note_id,
        serialized_data.note_assets,
        serialized_data.recipient,
        serialized_data.status,
        serialized_data.metadata,
        nullifier,
        serialized_data.details,
        serialized_data.note_script_hash,
        serialized_data.note_script,
        serialized_data.inclusion_proof,
        serialized_data.created_at,
        expected_height,
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

    let state = NoteState::read_from_bytes(&state)?;
    let created_at = created_at
        .parse::<u64>()
        .map_err(|_| StoreError::QueryError("Failed to parse created_at timestamp".to_string()))?;

    Ok(InputNoteRecord::new(details, Some(created_at), state))
}

pub fn parse_output_note_idxdb_object(
    note_idxdb: OutputNoteIdxdbObject,
) -> Result<OutputNoteRecord, StoreError> {
    let note_details: Option<NoteRecordDetails> =
        if let Some(details_as_json_bytes) = note_idxdb.details {
            let note_details: NoteRecordDetails =
                NoteRecordDetails::read_from_bytes(&details_as_json_bytes)?;

            Some(note_details)
        } else {
            None
        };
    let note_metadata = NoteMetadata::read_from_bytes(&note_idxdb.metadata)?;

    let note_assets = NoteAssets::read_from_bytes(&note_idxdb.assets)?;

    let inclusion_proof = match note_idxdb.inclusion_proof {
        Some(note_inclusion_proof) => {
            let note_inclusion_proof = NoteInclusionProof::read_from_bytes(&note_inclusion_proof)?;
            Some(note_inclusion_proof)
        },
        _ => None,
    };

    let recipient = Digest::try_from(note_idxdb.recipient)?;
    let id = NoteId::new(recipient, note_assets.commitment());

    let consumer_account_id: Option<AccountId> = match note_idxdb.consumer_account_id {
        Some(account_id) => Some(AccountId::from_hex(&account_id)?),
        None => None,
    };
    let created_at = note_idxdb.created_at.parse::<u64>().expect("Failed to parse created_at");
    let expected_height: Option<u32> = note_idxdb.expected_height.map(|expected_height| {
        expected_height.parse::<u32>().expect("Failed to parse expected_height")
    });
    let submitted_at: Option<u64> = note_idxdb
        .submitted_at
        .map(|submitted_at| submitted_at.parse::<u64>().expect("Failed to parse submitted_at"));
    let nullifier_height: Option<u32> = note_idxdb.nullifier_height.map(|nullifier_height| {
        nullifier_height.parse::<u32>().expect("Failed to parse nullifier_height")
    });

    // If the note is committed and has a consumer account id, then it was consumed locally but the
    // client is not synced with the chain
    let status = match note_idxdb.status.as_str() {
        NOTE_STATUS_EXPECTED => NoteStatus::Expected {
            created_at: Some(created_at),
            block_height: expected_height,
        },
        NOTE_STATUS_COMMITTED => NoteStatus::Committed {
            block_height: inclusion_proof
                .clone()
                .map(|proof| proof.location().block_num())
                .expect("Committed note should have inclusion proof"),
        },
        NOTE_STATUS_PROCESSING => NoteStatus::Processing {
            consumer_account_id: consumer_account_id
                .expect("Processing note should have consumer account id"),
            submitted_at: submitted_at.expect("Processing note should have submition timestamp"),
        },
        NOTE_STATUS_CONSUMED => NoteStatus::Consumed {
            consumer_account_id,
            block_height: nullifier_height.expect("Consumed note should have nullifier height"),
        },
        _ => {
            return Err(StoreError::DataDeserializationError(DeserializationError::InvalidValue(
                format!("NoteStatus: {}", note_idxdb.status),
            )))
        },
    };

    Ok(OutputNoteRecord::new(
        id,
        recipient,
        note_assets,
        status,
        note_metadata,
        inclusion_proof,
        note_details,
    ))
}

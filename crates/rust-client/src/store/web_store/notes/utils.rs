use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use chrono::Utc;
use miden_objects::{
    accounts::AccountId,
    notes::{NoteAssets, NoteId, NoteInclusionProof, NoteMetadata, NoteScript, NoteTag},
    transaction::TransactionId,
    utils::Deserializable,
    Digest,
};
use miden_tx::utils::{DeserializationError, Serializable};
use wasm_bindgen_futures::*;

use super::{js_bindings::*, InputNoteIdxdbObject, OutputNoteIdxdbObject};
use crate::store::{
    note_record::{
        NOTE_STATUS_COMMITTED, NOTE_STATUS_CONSUMED, NOTE_STATUS_EXPECTED, NOTE_STATUS_PROCESSING,
    },
    InputNoteRecord, NoteRecordDetails, NoteStatus, OutputNoteRecord, StoreError,
};

// TYPES
// ================================================================================================

pub struct SerializedInputNoteData {
    pub note_id: String,
    pub note_assets: Vec<u8>,
    pub recipient: String,
    pub status: String,
    pub metadata: Option<String>,
    pub details: String,
    pub note_script_hash: String,
    pub note_script: Vec<u8>,
    pub inclusion_proof: Option<String>,
    pub created_at: String,
    pub expected_height: Option<String>,
    pub ignored: bool,
    pub imported_tag: Option<String>,
    pub nullifier_height: Option<String>,
}

pub struct SerializedOutputNoteData {
    pub note_id: String,
    pub note_assets: Vec<u8>,
    pub recipient: String,
    pub status: String,
    pub metadata: String,
    pub details: Option<String>,
    pub note_script_hash: Option<String>,
    pub note_script: Option<Vec<u8>>,
    pub inclusion_proof: Option<String>,
    pub created_at: String,
    pub expected_height: Option<String>,
}

// ================================================================================================

pub(crate) async fn update_note_consumer_tx_id(
    note_id: NoteId,
    consumer_tx_id: TransactionId,
) -> Result<(), StoreError> {
    let serialized_note_id = note_id.inner().to_string();
    let serialized_consumer_tx_id = consumer_tx_id.to_string();
    let serialized_submitted_at = Utc::now().timestamp().to_string();

    let promise = idxdb_update_note_consumer_tx_id(
        serialized_note_id,
        serialized_consumer_tx_id,
        serialized_submitted_at,
    );
    JsFuture::from(promise).await.unwrap();

    Ok(())
}

pub(crate) fn serialize_input_note(
    note: InputNoteRecord,
) -> Result<SerializedInputNoteData, StoreError> {
    let note_id = note.id().inner().to_string();
    let note_assets = note.assets().to_bytes();

    let inclusion_proof = match note.inclusion_proof() {
        Some(proof) => {
            let block_num = proof.location().block_num();
            let node_index = proof.location().node_index_in_block();

            let inclusion_proof = serde_json::to_string(&NoteInclusionProof::new(
                block_num,
                node_index,
                proof.note_path().clone(),
            )?)
            .map_err(StoreError::InputSerializationError)?;

            Some(inclusion_proof)
        },
        None => None,
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
    let note_script = note.details().script().to_bytes();
    let created_at = Utc::now().timestamp().to_string();
    let ignored = note.ignored();
    let imported_tag: Option<u32> = note.imported_tag().map(|tag| tag.into());
    let imported_tag_str: Option<String> = imported_tag.map(|tag| tag.to_string());

    let (status, expected_height, nullifier_height) = match note.status() {
        NoteStatus::Expected { block_height, .. } => {
            let block_height_as_str = block_height.map(|height| height.to_string());
            (NOTE_STATUS_EXPECTED.to_string(), block_height_as_str, None)
        },
        NoteStatus::Committed { .. } => (NOTE_STATUS_COMMITTED.to_string(), None, None),
        NoteStatus::Processing { .. } => {
            return Err(StoreError::DatabaseError(
                "Processing notes should not be imported".to_string(),
            ))
        },
        NoteStatus::Consumed { block_height, .. } => {
            let block_height_as_str = block_height.to_string();
            (NOTE_STATUS_CONSUMED.to_string(), None, Some(block_height_as_str))
        },
    };

    Ok(SerializedInputNoteData {
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
        ignored,
        imported_tag: imported_tag_str,
        nullifier_height,
    })
}

pub async fn insert_input_note_tx(block_num: u32, note: InputNoteRecord) -> Result<(), StoreError> {
    let serialized_data = serialize_input_note(note)?;
    let expected_height = serialized_data.expected_height.or(Some(block_num.to_string()));

    let promise = idxdb_insert_input_note(
        serialized_data.note_id,
        serialized_data.note_assets,
        serialized_data.recipient,
        serialized_data.status,
        serialized_data.metadata,
        serialized_data.details,
        serialized_data.note_script_hash,
        serialized_data.note_script,
        serialized_data.inclusion_proof,
        serialized_data.created_at,
        expected_height,
        serialized_data.ignored,
        serialized_data.imported_tag,
        serialized_data.nullifier_height,
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
            let block_num = proof.location().block_num();
            let node_index = proof.location().node_index_in_block();

            let inclusion_proof = serde_json::to_string(&NoteInclusionProof::new(
                block_num,
                node_index,
                proof.note_path().clone(),
            )?)
            .map_err(StoreError::InputSerializationError)?;

            let status = NOTE_STATUS_COMMITTED.to_string();

            (Some(inclusion_proof), status)
        },
        None => {
            let status = NOTE_STATUS_EXPECTED.to_string();

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

    let result = JsFuture::from(idxdb_insert_output_note(
        serialized_data.note_id,
        serialized_data.note_assets,
        serialized_data.recipient,
        serialized_data.status,
        serialized_data.metadata,
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
    let note_script = NoteScript::read_from_bytes(&note_idxdb.serialized_note_script)?;
    let note_details: NoteRecordDetails = serde_json::from_str(&note_idxdb.details)
        .map_err(StoreError::JsonDataDeserializationError)?;
    let note_details = NoteRecordDetails::new(
        note_details.nullifier().to_string(),
        note_script,
        note_details.inputs().clone(),
        note_details.serial_num(),
    );

    let note_metadata: Option<NoteMetadata> =
        if let Some(metadata_as_json_str) = note_idxdb.metadata {
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
            submitted_at: submitted_at.expect("REASON"),
        },
        NOTE_STATUS_CONSUMED => NoteStatus::Consumed {
            consumer_account_id,
            block_height: nullifier_height.expect("REASON"),
        },
        _ => {
            return Err(StoreError::DataDeserializationError(DeserializationError::InvalidValue(
                format!("NoteStatus: {}", note_idxdb.status),
            )))
        },
    };

    let imported_tag_as_u32: Option<u32> =
        note_idxdb.imported_tag.as_ref().and_then(|tag| tag.parse::<u32>().ok());

    Ok(InputNoteRecord::new(
        id,
        recipient,
        note_assets,
        status,
        note_metadata,
        inclusion_proof,
        note_details,
        note_idxdb.ignored,
        imported_tag_as_u32.map(NoteTag::from),
    ))
}

pub fn parse_output_note_idxdb_object(
    note_idxdb: OutputNoteIdxdbObject,
) -> Result<OutputNoteRecord, StoreError> {
    let note_details: Option<NoteRecordDetails> =
        if let Some(details_as_json_str) = note_idxdb.details {
            // Merge the info that comes from the input notes table and the notes script table
            let serialized_note_script = note_idxdb
                .serialized_note_script
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
    let note_metadata: NoteMetadata = serde_json::from_str(&note_idxdb.metadata)
        .map_err(StoreError::JsonDataDeserializationError)?;

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

use alloc::{
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;
use std::print;

use chrono::Utc;
use miden_objects::{
    accounts::AccountId,
    crypto::utils::{Deserializable, Serializable},
    notes::{NoteAssets, NoteId, NoteInclusionProof, NoteMetadata, NoteScript, NoteTag, Nullifier},
    transaction::TransactionId,
    Digest,
};
use miden_tx::utils::DeserializationError;
use rusqlite::{named_params, params, params_from_iter, types::Value, Transaction};

use super::SqliteStore;
use crate::store::{
    note_record::{
        NOTE_STATUS_COMMITTED, NOTE_STATUS_CONSUMED, NOTE_STATUS_EXPECTED, NOTE_STATUS_PROCESSING,
    },
    InputNoteRecord, NoteFilter, NoteRecordDetails, NoteStatus, OutputNoteRecord, StoreError,
};

fn insert_note_query(table_name: NoteTable) -> String {
    format!("\
    INSERT INTO {table_name}
        (note_id, assets, recipient, status, metadata, details, inclusion_proof, consumer_transaction_id, created_at, ignored, imported_tag, nullifier_height)
     VALUES (:note_id, :assets, :recipient, :status, json(:metadata), json(:details), json(:inclusion_proof), :consumer_transaction_id, unixepoch(current_timestamp), :ignored, :imported_tag, :nullifier_height);",
            table_name = table_name)
}

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
    Option<String>,
    bool,
    Option<u32>,
    Option<u32>,
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

type SerializedInputNoteParts = (
    Vec<u8>,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Vec<u8>,
    Option<i64>,
    u64,
    Option<u64>,
    Option<u32>,
    bool,
    Option<u32>,
);
type SerializedOutputNoteParts = (
    Vec<u8>,
    Option<String>,
    String,
    String,
    String,
    Option<String>,
    Option<Vec<u8>>,
    Option<i64>,
    u64,
    Option<u64>,
    Option<u32>,
);

// NOTE TABLE
// ================================================================================================

/// Represents a table in the SQL DB used to store notes based on their use case
enum NoteTable {
    InputNotes,
    OutputNotes,
}

impl fmt::Display for NoteTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteTable::InputNotes => write!(f, "input_notes"),
            NoteTable::OutputNotes => write!(f, "output_notes"),
        }
    }
}

// NOTE FILTER
// ================================================================================================

impl<'a> NoteFilter<'a> {
    /// Returns a [String] containing the query for this Filter
    fn to_query(&self, notes_table: NoteTable) -> String {
        let base = format!(
            "SELECT
                    note.assets,
                    note.details,
                    note.recipient,
                    note.status,
                    note.metadata,
                    note.inclusion_proof,
                    script.serialized_note_script,
                    tx.account_id,
                    note.created_at,
                    note.submitted_at,
                    note.nullifier_height,
                    note.ignored,
                    note.imported_tag
                    from {notes_table} AS note
                    LEFT OUTER JOIN notes_scripts AS script
                        ON note.details IS NOT NULL AND
                        json_extract(note.details, '$.script_hash') = script.script_hash
                    LEFT OUTER JOIN transactions AS tx
                        ON note.consumer_transaction_id IS NOT NULL AND
                        note.consumer_transaction_id = tx.id"
        );

        match self {
            NoteFilter::All => base,
            NoteFilter::Committed => {
                format!("{base} WHERE status = '{NOTE_STATUS_COMMITTED}' AND NOT(ignored)")
            },
            NoteFilter::Consumed => {
                format!("{base} WHERE status = '{NOTE_STATUS_CONSUMED}' AND NOT(ignored)")
            },
            NoteFilter::Expected => {
                format!("{base} WHERE status = '{NOTE_STATUS_EXPECTED}' AND NOT(ignored)")
            },
            NoteFilter::Processing => {
                format!("{base} WHERE status = '{NOTE_STATUS_PROCESSING}' AND NOT(ignored)")
            },
            NoteFilter::Ignored => format!("{base} WHERE ignored"),
            NoteFilter::Unique(_) | NoteFilter::List(_) => {
                format!("{base} WHERE note.note_id IN rarray(?)")
            },
        }
    }
}

// NOTES STORE METHODS
// --------------------------------------------------------------------------------------------

impl SqliteStore {
    pub(crate) fn get_input_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        let mut params = Vec::new();
        match filter {
            NoteFilter::Unique(note_id) => {
                let note_ids_list = vec![Value::Text(note_id.inner().to_string())];
                params.push(Rc::new(note_ids_list));
            },
            NoteFilter::List(note_ids) => {
                let note_ids_list = note_ids
                    .iter()
                    .map(|note_id| Value::Text(note_id.inner().to_string()))
                    .collect::<Vec<Value>>();

                params.push(Rc::new(note_ids_list))
            },
            _ => {},
        }
        let notes = self
            .db()
            .prepare(&filter.to_query(NoteTable::InputNotes))?
            .query_map(params_from_iter(params), parse_input_note_columns)
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).and_then(parse_input_note))
            .collect::<Result<Vec<InputNoteRecord>, _>>()?;

        match filter {
            NoteFilter::Unique(note_id) if notes.is_empty() => {
                return Err(StoreError::NoteNotFound(note_id));
            },
            NoteFilter::List(note_ids) if note_ids.len() != notes.len() => {
                let missing_note_id = note_ids
                    .iter()
                    .find(|&note_id| !notes.iter().any(|note_record| note_record.id() == *note_id))
                    .expect("should find one note id that wasn't retrieved by the db");
                return Err(StoreError::NoteNotFound(*missing_note_id));
            },
            _ => {},
        }
        Ok(notes)
    }

    /// Retrieves the output notes from the database
    pub(crate) fn get_output_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        let mut params = Vec::new();
        match filter {
            NoteFilter::Unique(note_id) => {
                let note_ids_list = vec![Value::Text(note_id.inner().to_string())];
                params.push(Rc::new(note_ids_list));
            },
            NoteFilter::List(note_ids) => {
                let note_ids_list = note_ids
                    .iter()
                    .map(|note_id| Value::Text(note_id.inner().to_string()))
                    .collect::<Vec<Value>>();

                params.push(Rc::new(note_ids_list))
            },
            _ => {},
        }
        let notes = self
            .db()
            .prepare(&filter.to_query(NoteTable::OutputNotes))?
            .query_map(params_from_iter(params), parse_output_note_columns)
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).and_then(parse_output_note))
            .collect::<Result<Vec<OutputNoteRecord>, _>>()?;

        match filter {
            NoteFilter::Unique(note_id) if notes.is_empty() => {
                return Err(StoreError::NoteNotFound(note_id));
            },
            NoteFilter::List(note_ids) if note_ids.len() != notes.len() => {
                let missing_note_id = note_ids
                    .iter()
                    .find(|&note_id| !notes.iter().any(|note_record| note_record.id() == *note_id))
                    .expect("should find one note id that wasn't retrieved by the db");
                return Err(StoreError::NoteNotFound(*missing_note_id));
            },
            _ => {},
        }
        Ok(notes)
    }

    pub(crate) fn insert_input_note(&self, note: InputNoteRecord) -> Result<(), StoreError> {
        let mut db = self.db();
        let tx = db.transaction()?;

        insert_input_note_tx(&tx, note)?;

        Ok(tx.commit()?)
    }

    pub(crate) fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        const QUERY: &str =
                "SELECT json_extract(details, '$.nullifier') FROM input_notes WHERE status IN rarray(?)";
        let unspent_filters = Rc::new(vec![
            Value::from(NOTE_STATUS_COMMITTED.to_string()),
            Value::from(NOTE_STATUS_PROCESSING.to_string()),
        ]);
        self.db()
            .prepare(QUERY)?
            .query_map([unspent_filters], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                result.map_err(|err| StoreError::ParsingError(err.to_string())).and_then(
                    |v: String| {
                        Digest::try_from(v).map(Nullifier::from).map_err(StoreError::HexParseError)
                    },
                )
            })
            .collect::<Result<Vec<Nullifier>, _>>()
    }

    /// Updates the inclusion proof of the input note with the provided ID
    pub fn update_note_inclusion_proof(
        &self,
        note_id: NoteId,
        inclusion_proof: NoteInclusionProof,
    ) -> Result<(), StoreError> {
        const QUERY: &str = "UPDATE input_notes SET inclusion_proof = json(:inclusion_proof), status = 'Committed' WHERE note_id = :note_id";

        self.db()
            .execute(
                QUERY,
                named_params! {
                    ":note_id": note_id.inner().to_string(),
                    ":inclusion_proof": serde_json::to_string(&inclusion_proof).map_err(StoreError::JsonDataDeserializationError)?,
                },
            )
            .map_err(|err| StoreError::QueryError(err.to_string()))?;

        Ok(())
    }

    /// Updates the metadata of the input note with the provided ID
    pub fn update_note_metadata(
        &self,
        note_id: NoteId,
        metadata: NoteMetadata,
    ) -> Result<(), StoreError> {
        const QUERY: &str =
            "UPDATE input_notes SET metadata = json(:metadata) WHERE note_id = :note_id";

        self.db()
            .execute(
                QUERY,
                named_params! {
                    ":note_id": note_id.inner().to_string(),
                    ":metadata": serde_json::to_string(&metadata).map_err(StoreError::JsonDataDeserializationError)?,
                },
            )
            .map_err(|err| StoreError::QueryError(err.to_string()))?;

        Ok(())
    }

    pub(crate) fn testing_get_tracked_note_tag_single_query(
        &self,
    ) -> Result<Vec<NoteTag>, StoreError> {
        const QUERY: &str = "SELECT
            CAST(json_each.value AS UNSIGNED BIG INT) AS tag_value
        FROM
            state_sync,
            json_each(state_sync.tags)

        UNION

        SELECT CAST(json_extract(metadata, '$.tag') AS UNSIGNED BIG INT) AS tag_value
        FROM input_notes
        WHERE metadata IS NOT NULL
          AND json_extract(metadata, '$.tag') IS NOT NULL

        UNION

        SELECT CAST(imported_tag AS UNSIGNED BIG INT) AS tag_value
        FROM input_notes
        WHERE imported_tag IS NOT NULL

        UNION

        SELECT
            CAST((((id >> 34) & 0xffff0000) | 0xc0000000) AS UNSIGNED BIG INT) AS tag_value
        FROM accounts
        WHERE id IS NOT NULL;";

        self.db()
            .prepare(QUERY)
            .unwrap()
            .query_map([], |row| row.get::<usize, i64>(0))
            .expect("no binding parameters used in query")
            .filter_map(|result| {
                match result {
                    Ok(id) if id != 0 => {
                        // Convert to NoteTag if the id is not 0
                        print!("id: {}", id.clone());
                        let tag = NoteTag::try_from(id as u64).expect("account id is valid");
                        Some(Ok(tag))
                    },
                    Ok(_) => None, // Exclude 0 values
                    Err(err) => Some(Err(StoreError::QueryError(err.to_string()))), // Handle errors
                }
            })
            .collect::<Result<Vec<NoteTag>, StoreError>>()
    }
}

// HELPERS
// ================================================================================================

/// Inserts the provided input note into the database
pub(super) fn insert_input_note_tx(
    tx: &Transaction<'_>,
    note: InputNoteRecord,
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
        inclusion_proof,
        ignored,
        imported_tag,
        nullifier_height,
    ) = serialize_input_note(note)?;

    tx.execute(
        &insert_note_query(NoteTable::InputNotes),
        named_params! {
            ":note_id": note_id,
            ":assets": assets,
            ":recipient": recipient,
            ":status": status,
            ":metadata": metadata,
            ":details": details,
            ":inclusion_proof": inclusion_proof,
            ":consumer_transaction_id": None::<String>,
            ":ignored": ignored,
            ":imported_tag": imported_tag,
            ":nullifier_height": nullifier_height
        },
    )?;

    const QUERY: &str =
        "INSERT OR REPLACE INTO notes_scripts (script_hash, serialized_note_script) VALUES (?, ?)";
    tx.execute(QUERY, params![note_script_hash, serialized_note_script,])
        .map_err(|err| StoreError::QueryError(err.to_string()))
        .map(|_| ())
}

/// Inserts the provided input note into the database
pub fn insert_output_note_tx(
    tx: &Transaction<'_>,
    note: &OutputNoteRecord,
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
        inclusion_proof,
    ) = serialize_output_note(note)?;

    tx.execute(
        &insert_note_query(NoteTable::OutputNotes),
        named_params! {
            ":note_id": note_id,
            ":assets": assets,
            ":recipient": recipient,
            ":status": status,
            ":metadata": metadata,
            ":details": details,
            ":inclusion_proof": inclusion_proof,
            ":ignored": false,
            ":imported_tag": None::<u32>,
        },
    )?;

    if note_script_hash.is_some() {
        const QUERY: &str =
            "INSERT OR REPLACE INTO notes_scripts (script_hash, serialized_note_script) VALUES (?, ?)";
        tx.execute(QUERY, params![note_script_hash, serialized_note_script,])?;
    }

    Ok(())
}

pub fn update_note_consumer_tx_id(
    tx: &Transaction<'_>,
    note_id: NoteId,
    consumer_tx_id: TransactionId,
) -> Result<(), StoreError> {
    const UPDATE_INPUT_NOTES_QUERY: &str = "UPDATE input_notes SET status = :status, consumer_transaction_id = :consumer_transaction_id, submitted_at = :submitted_at WHERE note_id = :note_id;";

    tx.execute(
        UPDATE_INPUT_NOTES_QUERY,
        named_params! {
            ":note_id": note_id.inner().to_string(),
            ":consumer_transaction_id": consumer_tx_id.to_string(),
            ":submitted_at": Utc::now().timestamp(),
            ":status": NOTE_STATUS_PROCESSING,
        },
    )?;

    const UPDATE_OUTPUT_NOTES_QUERY: &str = "UPDATE output_notes SET status = :status, consumer_transaction_id = :consumer_transaction_id, submitted_at = :submitted_at WHERE note_id = :note_id;";

    tx.execute(
        UPDATE_OUTPUT_NOTES_QUERY,
        named_params! {
            ":note_id": note_id.inner().to_string(),
            ":consumer_transaction_id": consumer_tx_id.to_string(),
            ":submitted_at": Utc::now().timestamp(),
            ":status": NOTE_STATUS_PROCESSING,
        },
    )?;

    Ok(())
}

/// Parse input note columns from the provided row into native types.
fn parse_input_note_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedInputNoteParts, rusqlite::Error> {
    let assets: Vec<u8> = row.get(0)?;
    let details: String = row.get(1)?;
    let recipient: String = row.get(2)?;
    let status: String = row.get(3)?;
    let metadata: Option<String> = row.get(4)?;
    let inclusion_proof: Option<String> = row.get(5)?;
    let serialized_note_script: Vec<u8> = row.get(6)?;
    let consumer_account_id: Option<i64> = row.get(7)?;
    let created_at: u64 = row.get(8)?;
    let submitted_at: Option<u64> = row.get(9)?;
    let nullifier_height: Option<u32> = row.get(10)?;
    let ignored: bool = row.get(8)?;
    let imported_tag: Option<u32> = row.get(9)?;

    Ok((
        assets,
        details,
        recipient,
        status,
        metadata,
        inclusion_proof,
        serialized_note_script,
        consumer_account_id,
        created_at,
        submitted_at,
        nullifier_height,
        ignored,
        imported_tag,
    ))
}

/// Parse a note from the provided parts.
fn parse_input_note(
    serialized_input_note_parts: SerializedInputNoteParts,
) -> Result<InputNoteRecord, StoreError> {
    let (
        note_assets,
        note_details,
        recipient,
        status,
        note_metadata,
        note_inclusion_proof,
        serialized_note_script,
        consumer_account_id,
        created_at,
        submitted_at,
        nullifier_height,
        ignored,
        imported_tag,
    ) = serialized_input_note_parts;

    // Merge the info that comes from the input notes table and the notes script table
    let note_script = NoteScript::read_from_bytes(&serialized_note_script)?;
    let note_details: NoteRecordDetails =
        serde_json::from_str(&note_details).map_err(StoreError::JsonDataDeserializationError)?;
    let note_details = NoteRecordDetails::new(
        note_details.nullifier().to_string(),
        note_script,
        note_details.inputs().clone(),
        note_details.serial_num(),
    );

    let note_metadata: Option<NoteMetadata> = if let Some(metadata_as_json_str) = note_metadata {
        Some(
            serde_json::from_str(&metadata_as_json_str)
                .map_err(StoreError::JsonDataDeserializationError)?,
        )
    } else {
        None
    };

    let note_assets = NoteAssets::read_from_bytes(&note_assets)?;

    let inclusion_proof = match note_inclusion_proof {
        Some(note_inclusion_proof) => {
            let note_inclusion_proof: NoteInclusionProof =
                serde_json::from_str(&note_inclusion_proof)
                    .map_err(StoreError::JsonDataDeserializationError)?;

            Some(note_inclusion_proof)
        },
        _ => None,
    };

    let recipient = Digest::try_from(recipient)?;
    let id = NoteId::new(recipient, note_assets.commitment());
    let consumer_account_id: Option<AccountId> = match consumer_account_id {
        Some(account_id) => Some(AccountId::try_from(account_id as u64)?),
        None => None,
    };

    // If the note is committed and has a consumer account id, then it was consumed locally but the
    // client is not synced with the chain
    let status = match status.as_str() {
        NOTE_STATUS_EXPECTED => NoteStatus::Expected { created_at: Some(created_at) },
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
                format!("NoteStatus: {}", status),
            )))
        },
    };

    Ok(InputNoteRecord::new(
        id,
        recipient,
        note_assets,
        status,
        note_metadata,
        inclusion_proof,
        note_details,
        ignored,
        imported_tag.map(NoteTag::from),
    ))
}

/// Serialize the provided input note into database compatible types.
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
    let serialized_note_script = note.details().script().to_bytes();

    let ignored = note.ignored();

    let imported_tag: Option<u32> = note.imported_tag().map(|tag| tag.into());

    let (status, nullifier_height) = match note.status() {
        NoteStatus::Expected { .. } => (NOTE_STATUS_EXPECTED.to_string(), None),
        NoteStatus::Committed { .. } => (NOTE_STATUS_COMMITTED.to_string(), None),
        NoteStatus::Processing { .. } => {
            return Err(StoreError::DatabaseError(
                "Processing notes should not be imported".to_string(),
            ))
        },
        NoteStatus::Consumed { block_height, .. } => {
            (NOTE_STATUS_CONSUMED.to_string(), Some(block_height))
        },
    };

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
        ignored,
        imported_tag,
        nullifier_height,
    ))
}

/// Parse input note columns from the provided row into native types.
fn parse_output_note_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedOutputNoteParts, rusqlite::Error> {
    let assets: Vec<u8> = row.get(0)?;
    let details: Option<String> = row.get(1)?;
    let recipient: String = row.get(2)?;
    let status: String = row.get(3)?;
    let metadata: String = row.get(4)?;
    let inclusion_proof: Option<String> = row.get(5)?;
    let serialized_note_script: Option<Vec<u8>> = row.get(6)?;
    let consumer_account_id: Option<i64> = row.get(7)?;
    let created_at: u64 = row.get(8)?;
    let submitted_at: Option<u64> = row.get(9)?;
    let nullifier_height: Option<u32> = row.get(10)?;

    Ok((
        assets,
        details,
        recipient,
        status,
        metadata,
        inclusion_proof,
        serialized_note_script,
        consumer_account_id,
        created_at,
        submitted_at,
        nullifier_height,
    ))
}

/// Parse a note from the provided parts.
fn parse_output_note(
    serialized_output_note_parts: SerializedOutputNoteParts,
) -> Result<OutputNoteRecord, StoreError> {
    let (
        note_assets,
        note_details,
        recipient,
        status,
        note_metadata,
        note_inclusion_proof,
        serialized_note_script,
        consumer_account_id,
        created_at,
        submitted_at,
        nullifier_height,
    ) = serialized_output_note_parts;

    let note_details: Option<NoteRecordDetails> = if let Some(details_as_json_str) = note_details {
        // Merge the info that comes from the input notes table and the notes script table
        let serialized_note_script = serialized_note_script
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
        serde_json::from_str(&note_metadata).map_err(StoreError::JsonDataDeserializationError)?;

    let note_assets = NoteAssets::read_from_bytes(&note_assets)?;

    let inclusion_proof = match note_inclusion_proof {
        Some(note_inclusion_proof) => {
            let note_inclusion_proof: NoteInclusionProof =
                serde_json::from_str(&note_inclusion_proof)
                    .map_err(StoreError::JsonDataDeserializationError)?;

            Some(note_inclusion_proof)
        },
        _ => None,
    };

    let recipient = Digest::try_from(recipient)?;
    let id = NoteId::new(recipient, note_assets.commitment());

    let consumer_account_id: Option<AccountId> = match consumer_account_id {
        Some(account_id) => Some(AccountId::try_from(account_id as u64)?),
        None => None,
    };

    // If the note is committed and has a consumer account id, then it was consumed locally but the
    // client is not synced with the chain
    let status = match status.as_str() {
        NOTE_STATUS_EXPECTED => NoteStatus::Expected { created_at: Some(created_at) },
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
                format!("NoteStatus: {}", status),
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

/// Serialize the provided output note into database compatible types.
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

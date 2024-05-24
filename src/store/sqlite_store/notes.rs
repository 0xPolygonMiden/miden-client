use alloc::rc::Rc;
use std::fmt;

use clap::error::Result;
use miden_objects::{
    accounts::AccountId,
    crypto::utils::{Deserializable, Serializable},
    notes::{NoteAssets, NoteId, NoteInclusionProof, NoteMetadata, NoteScript, Nullifier},
    transaction::TransactionId,
    Digest,
};
use rusqlite::{named_params, params, params_from_iter, types::Value, Transaction};

use super::SqliteStore;
use crate::{
    errors::StoreError,
    store::{InputNoteRecord, NoteFilter, NoteRecordDetails, NoteStatus, OutputNoteRecord},
};

fn insert_note_query(table_name: NoteTable) -> String {
    format!("\
    INSERT INTO {table_name}
        (note_id, assets, recipient, status, metadata, details, inclusion_proof, consumer_transaction_id) 
     VALUES (:note_id, :assets, :recipient, :status, json(:metadata), json(:details), json(:inclusion_proof), :consumer_transaction_id)",
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
                    tx.account_id
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
            NoteFilter::Committed => format!("{base} WHERE status = 'Committed'"),
            NoteFilter::Consumed => format!("{base} WHERE status = 'Consumed'"),
            NoteFilter::Pending => format!("{base} WHERE status = 'Pending'"),
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

    pub(crate) fn insert_input_note(&self, note: &InputNoteRecord) -> Result<(), StoreError> {
        let mut db = self.db();
        let tx = db.transaction()?;

        insert_input_note_tx(&tx, note)?;

        Ok(tx.commit()?)
    }

    /// Returns the nullifiers of all unspent input notes
    pub fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        const QUERY: &str = "SELECT json_extract(details, '$.nullifier') FROM input_notes WHERE status = 'Committed'";

        self.db()
            .prepare(QUERY)?
            .query_map([], |row| row.get(0))
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
}

// HELPERS
// ================================================================================================

/// Inserts the provided input note into the database
pub(super) fn insert_input_note_tx(
    tx: &Transaction<'_>,
    note: &InputNoteRecord,
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
        },
    )
    .map_err(|err| StoreError::QueryError(err.to_string()))
    .map(|_| ())?;

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
        },
    )
    .map_err(|err| StoreError::QueryError(err.to_string()))
    .map(|_| ())?;

    const QUERY: &str =
        "INSERT OR REPLACE INTO notes_scripts (script_hash, serialized_note_script) VALUES (?, ?)";
    tx.execute(QUERY, params![note_script_hash, serialized_note_script,])
        .map_err(|err| StoreError::QueryError(err.to_string()))
        .map(|_| ())
}

pub fn update_note_consumer_tx_id(
    tx: &Transaction<'_>,
    note_id: NoteId,
    consumer_tx_id: TransactionId,
) -> Result<(), StoreError> {
    const UPDATE_INPUT_NOTES_QUERY: &str = "UPDATE input_notes SET consumer_transaction_id = :consumer_transaction_id WHERE note_id = :note_id;";

    tx.execute(
        UPDATE_INPUT_NOTES_QUERY,
        named_params! {
            ":note_id": note_id.inner().to_string(),
            ":consumer_transaction_id": consumer_tx_id.to_string(),
        },
    )
    .map_err(|err| StoreError::QueryError(err.to_string()))?;

    const UPDATE_OUTPUT_NOTES_QUERY: &str = "UPDATE output_notes SET consumer_transaction_id = :consumer_transaction_id WHERE note_id = :note_id;";

    tx.execute(
        UPDATE_OUTPUT_NOTES_QUERY,
        named_params! {
            ":note_id": note_id.inner().to_string(),
            ":consumer_transaction_id": consumer_tx_id.to_string(),
        },
    )
    .map_err(|err| StoreError::QueryError(err.to_string()))?;

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

    Ok((
        assets,
        details,
        recipient,
        status,
        metadata,
        inclusion_proof,
        serialized_note_script,
        consumer_account_id,
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
    let status: NoteStatus = serde_json::from_str(&format!("\"{status}\""))
        .map_err(StoreError::JsonDataDeserializationError)?;
    let consumer_account_id: Option<AccountId> = match consumer_account_id {
        Some(account_id) => Some(AccountId::try_from(account_id as u64)?),
        None => None,
    };
    Ok(InputNoteRecord::new(
        id,
        recipient,
        note_assets,
        status,
        note_metadata,
        inclusion_proof,
        note_details,
        consumer_account_id,
    ))
}

/// Serialize the provided input note into database compatible types.
pub(crate) fn serialize_input_note(
    note: &InputNoteRecord,
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

    Ok((
        assets,
        details,
        recipient,
        status,
        metadata,
        inclusion_proof,
        serialized_note_script,
        consumer_account_id,
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
    let status: NoteStatus = serde_json::from_str(&format!("\"{status}\""))
        .map_err(StoreError::JsonDataDeserializationError)?;

    let consumer_account_id: Option<AccountId> = match consumer_account_id {
        Some(account_id) => Some(AccountId::try_from(account_id as u64)?),
        None => None,
    };

    Ok(OutputNoteRecord::new(
        id,
        recipient,
        note_assets,
        status,
        note_metadata,
        inclusion_proof,
        note_details,
        consumer_account_id,
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

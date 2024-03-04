use std::fmt;

use crate::errors::StoreError;
use crate::store::{InputNoteRecord, NoteFilter, NoteRecordDetails, NoteStatus, OutputNoteRecord};

use super::SqliteStore;

use clap::error::Result;

use crypto::utils::{Deserializable, Serializable};

use objects::notes::{NoteAssets, NoteId, NoteInclusionProof, NoteMetadata, Nullifier};

use objects::Digest;
use rusqlite::{named_params, params, Transaction};

fn insert_note_query(table_name: NoteTable) -> String {
    format!("\
    INSERT INTO {table_name}
        (note_id, assets, recipient, status, metadata, details, inclusion_proof) 
     VALUES (:note_id, :assets, :recipient, :status, json(:metadata), json(:details), json(:inclusion_proof))")
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
);

type SerializedInputNoteParts = (
    Vec<u8>,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
);
type SerializedOutputNoteParts = (
    Vec<u8>,
    Option<String>,
    String,
    String,
    String,
    Option<String>,
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

impl NoteFilter {
    /// Returns a [String] containing the query for this Filter
    fn to_query(&self, notes_table: NoteTable) -> String {
        let base = format!(
            "SELECT 
                        assets, 
                        details, 
                        recipient,
                        status,
                        metadata,
                        inclusion_proof
                        from {notes_table}"
        );

        match self {
            NoteFilter::All => base,
            NoteFilter::Committed => format!("{base} WHERE status = '\"Committed\"'"),
            NoteFilter::Consumed => format!("{base} WHERE status = '\"Consumed\"'"),
            NoteFilter::Pending => format!("{base} WHERE status = '\"Pending\"'"),
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
        self.db
            .prepare(&filter.to_query(NoteTable::InputNotes))?
            .query_map([], parse_input_note_columns)
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).and_then(parse_input_note))
            .collect::<Result<Vec<InputNoteRecord>, _>>()
    }

    /// Retrieves the output notes from the database
    pub(crate) fn get_output_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        self.db
            .prepare(&filter.to_query(NoteTable::OutputNotes))?
            .query_map([], parse_output_note_columns)
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).and_then(parse_output_note))
            .collect::<Result<Vec<OutputNoteRecord>, _>>()
    }

    pub(crate) fn get_input_note(&self, note_id: NoteId) -> Result<InputNoteRecord, StoreError> {
        let query_id = &note_id.inner().to_string();

        const QUERY: &str = "SELECT 
                                    assets, 
                                    details, 
                                    recipient,
                                    status,
                                    metadata,
                                    inclusion_proof
                                    from input_notes WHERE note_id = ?";

        self.db
            .prepare(QUERY)?
            .query_map(params![query_id.to_string()], parse_input_note_columns)?
            .map(|result| Ok(result?).and_then(parse_input_note))
            .next()
            .ok_or(StoreError::InputNoteNotFound(note_id))?
    }

    pub(crate) fn insert_input_note(&mut self, note: &InputNoteRecord) -> Result<(), StoreError> {
        let tx = self.db.transaction()?;

        insert_input_note_tx(&tx, note)?;

        Ok(tx.commit()?)
    }

    /// Returns the nullifiers of all unspent input notes
    pub fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        const QUERY: &str = "SELECT json_extract(details, '$.nullifier') FROM input_notes WHERE status = '\"Committed\"'";

        self.db
            .prepare(QUERY)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(|err| StoreError::ParsingError(err.to_string()))
                    .and_then(|v: String| {
                        Digest::try_from(v)
                            .map(Nullifier::from)
                            .map_err(StoreError::HexParseError)
                    })
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
    let (note_id, assets, recipient, status, metadata, details, inclusion_proof) =
        serialize_input_note(note)?;

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
        },
    )
    .map_err(|err| StoreError::QueryError(err.to_string()))
    .map(|_| ())
}

/// Inserts the provided input note into the database
pub fn insert_output_note_tx(
    tx: &Transaction<'_>,
    note: &OutputNoteRecord,
) -> Result<(), StoreError> {
    let (note_id, assets, recipient, status, metadata, details, inclusion_proof) =
        serialize_output_note(note)?;

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
    .map(|_| ())
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

    Ok((
        assets,
        details,
        recipient,
        status,
        metadata,
        inclusion_proof,
    ))
}

/// Parse a note from the provided parts.
fn parse_input_note(
    serialized_input_note_parts: SerializedInputNoteParts,
) -> Result<InputNoteRecord, StoreError> {
    let (note_assets, note_details, recipient, status, note_metadata, note_inclusion_proof) =
        serialized_input_note_parts;

    let note_details: NoteRecordDetails = dbg!(
        serde_json::from_str(&note_details).map_err(StoreError::JsonDataDeserializationError)
    )?;

    let note_metadata: Option<NoteMetadata> = if let Some(metadata_as_json_str) = note_metadata {
        Some(dbg!(serde_json::from_str(&metadata_as_json_str)
            .map_err(StoreError::JsonDataDeserializationError))?)
    } else {
        None
    };

    let note_assets = NoteAssets::read_from_bytes(&note_assets)?;

    let inclusion_proof = match note_inclusion_proof {
        Some(note_inclusion_proof) => {
            let note_inclusion_proof: NoteInclusionProof =
                dbg!(serde_json::from_str(&note_inclusion_proof)
                    .map_err(StoreError::JsonDataDeserializationError))?;

            Some(note_inclusion_proof)
        }
        _ => None,
    };

    let recipient = Digest::try_from(recipient)?;
    let id = NoteId::new(recipient, note_assets.commitment());
    let status: NoteStatus =
        dbg!(serde_json::from_str(&status).map_err(StoreError::JsonDataDeserializationError))?;

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

/// Serialize the provided input note into database compatible types.
pub(crate) fn serialize_input_note(
    note: &InputNoteRecord,
) -> Result<SerializedInputNoteData, StoreError> {
    let note_id = note.id().inner().to_string();
    let note_assets = note.assets().to_bytes();
    let (inclusion_proof, status) = match note.inclusion_proof() {
        Some(proof) => {
            // FIXME: This removal is to accomodate a problem with how the node constructs paths where
            // they are constructed using note ID instead of authentication hash, so for now we remove the first
            // node here.
            //
            // Note: once removed we can also stop creating a new `NoteInclusionProof`
            //
            // See: https://github.com/0xPolygonMiden/miden-node/blob/main/store/src/state.rs#L274
            let mut path = proof.note_path().clone();
            if path.len() > 0 {
                let _removed = path.remove(0);
            }

            let block_num = proof.origin().block_num;
            let node_index = proof.origin().node_index.value();
            let sub_hash = proof.sub_hash();
            let note_root = proof.note_root();

            let inclusion_proof = serde_json::to_string(&NoteInclusionProof::new(
                block_num, sub_hash, note_root, node_index, path,
            )?)
            .map_err(StoreError::InputSerializationError)?;

            let status = serde_json::to_string(&NoteStatus::Committed)
                .map_err(StoreError::InputSerializationError)?;

            (Some(inclusion_proof), status)
        }
        None => {
            let status = serde_json::to_string(&NoteStatus::Pending)
                .map_err(StoreError::InputSerializationError)?;

            (None, status)
        }
    };
    let recipient = note.recipient().to_hex();

    let metadata = if let Some(metadata) = note.metadata() {
        Some(serde_json::to_string(metadata).map_err(StoreError::InputSerializationError)?)
    } else {
        None
    };

    let details =
        serde_json::to_string(&note.details()).map_err(StoreError::InputSerializationError)?;

    Ok((
        note_id,
        note_assets,
        recipient,
        status,
        metadata,
        details,
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

    Ok((
        assets,
        details,
        recipient,
        status,
        metadata,
        inclusion_proof,
    ))
}

/// Parse a note from the provided parts.
fn parse_output_note(
    serialized_output_note_parts: SerializedOutputNoteParts,
) -> Result<OutputNoteRecord, StoreError> {
    let (note_assets, note_details, recipient, status, note_metadata, note_inclusion_proof) =
        serialized_output_note_parts;

    let note_details: Option<NoteRecordDetails> = if let Some(details_as_json_str) = note_details {
        Some(
            serde_json::from_str(&details_as_json_str)
                .map_err(StoreError::JsonDataDeserializationError)?,
        )
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
        }
        _ => None,
    };

    let recipient = Digest::try_from(recipient)?;
    let id = NoteId::new(recipient, note_assets.commitment());
    let status: NoteStatus =
        serde_json::from_str(&status).map_err(StoreError::JsonDataDeserializationError)?;

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
            // FIXME: This removal is to accomodate a problem with how the node constructs paths where
            // they are constructed using note ID instead of authentication hash, so for now we remove the first
            // node here.
            //
            // Note: once removed we can also stop creating a new `NoteInclusionProof`
            //
            // See: https://github.com/0xPolygonMiden/miden-node/blob/main/store/src/state.rs#L274
            let mut path = proof.note_path().clone();
            if path.len() > 0 {
                let _removed = path.remove(0);
            }

            let block_num = proof.origin().block_num;
            let node_index = proof.origin().node_index.value();
            let sub_hash = proof.sub_hash();
            let note_root = proof.note_root();

            let inclusion_proof = serde_json::to_string(&NoteInclusionProof::new(
                block_num, sub_hash, note_root, node_index, path,
            )?)
            .map_err(StoreError::InputSerializationError)?;

            (Some(inclusion_proof), String::from("committed"))
        }
        None => (None, String::from("pending")),
    };
    let recipient = note.recipient().to_hex();

    let sender_id = note.metadata().sender();
    let tag = note.metadata().tag();
    let metadata = serde_json::to_string(&NoteMetadata::new(sender_id, tag))
        .map_err(StoreError::InputSerializationError)?;

    let details = if let Some(details) = note.details() {
        Some(serde_json::to_string(&details).map_err(StoreError::InputSerializationError)?)
    } else {
        None
    };

    Ok((
        note_id,
        note_assets,
        recipient,
        status,
        metadata,
        details,
        inclusion_proof,
    ))
}

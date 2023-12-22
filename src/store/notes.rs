use crate::errors::StoreError;

use super::Store;

use clap::error::Result;
use crypto::utils::{Deserializable, Serializable};
use crypto::Word;
use objects::notes::NoteScript;
use objects::notes::{Note, NoteInputs, NoteVault};

use objects::{
    accounts::AccountId,
    notes::{NoteMetadata, RecordedNote},
    Digest, Felt,
};
use rusqlite::params;

// TYPES
// ================================================================================================

type SerializedInputNoteData = (
    String,
    String,
    Vec<u8>,
    String,
    String,
    String,
    i64,
    i64,
    i64,
    Option<String>,
    String,
    String,
    u32,
);

type SerializedInputNoteParts = (
    Vec<u8>,
    String,
    String,
    String,
    u64,
    u64,
    u64,
    Option<String>,
);

// NOTE FILTER
// ================================================================================================
/// Represents a filter for input notes
pub enum InputNoteFilter {
    All,
    Consumed,
    Committed,
    Pending,
}

impl InputNoteFilter {
    pub fn to_query(&self) -> String {
        let base = String::from("SELECT script, inputs, vault, serial_num, sender_id, tag, num_assets, inclusion_proof FROM input_notes");
        match self {
            InputNoteFilter::All => base,
            InputNoteFilter::Committed => format!("{base} WHERE status = 'committed'"),
            InputNoteFilter::Consumed => format!("{base} WHERE status = 'consumed'"),
            InputNoteFilter::Pending => format!("{base} WHERE status = 'pending'"),
        }
    }
}

#[derive(Debug)]
pub enum NoteType {
    PendingNote(Note),
    CommittedNote(RecordedNote),
}

impl NoteType {
    pub fn hash(&self) -> Digest {
        match self {
            NoteType::PendingNote(n) => n.hash(),
            NoteType::CommittedNote(n) => n.note().hash(),
        }
    }

    pub fn script(&self) -> &NoteScript {
        match self {
            NoteType::PendingNote(n) => n.script(),
            NoteType::CommittedNote(n) => n.note().script(),
        }
    }

    pub fn vault(&self) -> &NoteVault {
        match self {
            NoteType::PendingNote(n) => n.vault(),
            NoteType::CommittedNote(n) => n.note().vault(),
        }
    }

    pub fn inputs(&self) -> &NoteInputs {
        match self {
            NoteType::PendingNote(n) => n.inputs(),
            NoteType::CommittedNote(n) => n.note().inputs(),
        }
    }

    pub fn serial_num(&self) -> Word {
        match self {
            NoteType::PendingNote(n) => n.serial_num(),
            NoteType::CommittedNote(n) => n.note().serial_num(),
        }
    }
}

impl Store {
    // NOTES
    // --------------------------------------------------------------------------------------------

    /// Retrieves the input notes from the database
    pub fn get_input_notes(
        &self,
        note_filter: InputNoteFilter,
    ) -> Result<Vec<NoteType>, StoreError> {
        self.db
            .prepare(&note_filter.to_query())
            .map_err(StoreError::QueryError)?
            .query_map([], parse_input_note_columns)
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_input_note)
            })
            .collect::<Result<Vec<NoteType>, _>>()
    }

    /// Retrieves pending (ie, not committed/consumed) note hashes
    pub fn get_pending_note_hashes(&self) -> Result<Vec<Digest>, StoreError> {
        self.db
            .prepare(&InputNoteFilter::Pending.to_query())
            .map_err(StoreError::QueryError)?
            .query_map([], parse_input_note_columns)
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_input_note)
            })
            .filter_map(|note| {
                if let Ok(NoteType::PendingNote(inner_note)) = note {
                    Some(Ok(inner_note.hash()))
                } else {
                    None
                }
            })
            .collect::<Result<Vec<Digest>, _>>()
    }

    /// Retrieves pending (ie, not committed/consumed) note hashes
    pub fn get_recorded_notes(&self) -> Result<Vec<RecordedNote>, StoreError> {
        self.db
            .prepare(&InputNoteFilter::All.to_query())
            .map_err(StoreError::QueryError)?
            .query_map([], parse_input_note_columns)
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_input_note)
            })
            .filter_map(|note| {
                if let Ok(NoteType::CommittedNote(inner_note)) = note {
                    Some(Ok(inner_note))
                } else {
                    None
                }
            })
            .collect::<Result<Vec<RecordedNote>, _>>()
    }

    /// Retrieves the input note with the specified hash from the database
    pub fn get_input_note_by_hash(&self, hash: Digest) -> Result<NoteType, StoreError> {
        let query_hash = &hash.to_string();
        const QUERY: &str = "SELECT script, inputs, vault, serial_num, sender_id, tag, num_assets, inclusion_proof FROM input_notes WHERE hash = ?";

        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map(params![query_hash.to_string()], parse_input_note_columns)
            .map_err(StoreError::QueryError)?
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_input_note)
            })
            .next()
            .ok_or(StoreError::InputNoteNotFound(hash))?
    }

    /// Inserts the provided input note into the database
    pub fn insert_input_note(&self, recorded_note: &RecordedNote) -> Result<(), StoreError> {
        let (
            hash,
            nullifier,
            script,
            vault,
            inputs,
            serial_num,
            sender_id,
            tag,
            num_assets,
            inclusion_proof,
            recipients,
            status,
            commit_height,
        ) = serialize_input_note(recorded_note)?;

        const QUERY: &str = "\
        INSERT INTO input_notes
            (hash, nullifier, script, vault, inputs, serial_num, sender_id, tag, num_assets, inclusion_proof, recipients, status, commit_height)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

        self.db
            .execute(
                QUERY,
                params![
                    hash,
                    nullifier,
                    script,
                    vault,
                    inputs,
                    serial_num,
                    sender_id,
                    tag,
                    num_assets,
                    inclusion_proof,
                    recipients,
                    status,
                    commit_height
                ],
            )
            .map_err(StoreError::QueryError)
            .map(|_| ())
    }

    /// Inserts the provided Note (that has not yet been committed to the network) into the database with a pending status
    pub fn insert_pending_note(&self, note: &Note) -> Result<(), StoreError> {
        let (
            hash,
            nullifier,
            script,
            vault,
            inputs,
            serial_num,
            sender_id,
            tag,
            num_assets,
            inclusion_proof,
            recipients,
            status,
            commit_height,
        ) = serialize_pending_note(note)?;

        const QUERY: &str = "\
        INSERT INTO input_notes
            (hash, nullifier, script, vault, inputs, serial_num, sender_id, tag, num_assets, inclusion_proof, recipients, status, commit_height)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

        self.db
            .execute(
                QUERY,
                params![
                    hash,
                    nullifier,
                    script,
                    vault,
                    inputs,
                    serial_num,
                    sender_id,
                    tag,
                    num_assets,
                    inclusion_proof,
                    recipients,
                    status,
                    commit_height
                ],
            )
            .map_err(StoreError::QueryError)
            .map(|_| ())
    }

    /// Returns the nullifiers of all unspent input notes
    pub fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Digest>, StoreError> {
        const QUERY: &str = "SELECT nullifier FROM input_notes WHERE status = 'committed'";

        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(|v: String| Digest::try_from(v).map_err(StoreError::HexParseError))
            })
            .collect::<Result<Vec<Digest>, _>>()
    }
}
// HELPERS
// ================================================================================================

/// Parse input note columns from the provided row into native types.
fn parse_input_note_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedInputNoteParts, rusqlite::Error> {
    let script: Vec<u8> = row.get(0)?;
    let inputs: String = row.get(1)?;
    let vault: String = row.get(2)?;
    let serial_num: String = row.get(3)?;
    let sender_id = row.get::<usize, i64>(4)? as u64;
    let tag = row.get::<usize, i64>(5)? as u64;
    let num_assets = row.get::<usize, i64>(6)? as u64;
    let inclusion_proof: Option<String> = row.get(7)?;
    Ok((
        script,
        inputs,
        vault,
        serial_num,
        sender_id,
        tag,
        num_assets,
        inclusion_proof,
    ))
}

/// Parse a note from the provided parts.
fn parse_input_note(
    serialized_input_note_parts: SerializedInputNoteParts,
) -> Result<NoteType, StoreError> {
    let (script, inputs, vault, serial_num, sender_id, tag, num_assets, inclusion_proof) =
        serialized_input_note_parts;
    let script =
        NoteScript::read_from_bytes(&script).map_err(StoreError::DataDeserializationError)?;
    let inputs = serde_json::from_str(&inputs).map_err(StoreError::JsonDataDeserializationError)?;
    let vault = serde_json::from_str(&vault).map_err(StoreError::JsonDataDeserializationError)?;
    let serial_num =
        serde_json::from_str(&serial_num).map_err(StoreError::JsonDataDeserializationError)?;
    let note_metadata = NoteMetadata::new(
        AccountId::new_unchecked(Felt::new(sender_id)),
        Felt::new(tag),
        Felt::new(num_assets),
    );
    let note = Note::from_parts(script, inputs, vault, serial_num, note_metadata);

    match inclusion_proof {
        Some(proof) => {
            let inclusion_proof =
                serde_json::from_str(&proof).map_err(StoreError::JsonDataDeserializationError)?;
            Ok(NoteType::CommittedNote(RecordedNote::new(
                note,
                inclusion_proof,
            )))
        }
        None => Ok(NoteType::PendingNote(note)),
    }
}

/// Serialize the provided input note into database compatible types.
fn serialize_input_note(
    recorded_note: &RecordedNote,
) -> Result<SerializedInputNoteData, StoreError> {
    let hash = recorded_note.note().hash().to_string();
    let nullifier = recorded_note.note().nullifier().inner().to_string();
    let script = recorded_note.note().script().to_bytes();
    let vault = serde_json::to_string(&recorded_note.note().vault())
        .map_err(StoreError::InputSerializationError)?;
    let inputs = serde_json::to_string(&recorded_note.note().inputs())
        .map_err(StoreError::InputSerializationError)?;
    let serial_num = serde_json::to_string(&recorded_note.note().serial_num())
        .map_err(StoreError::InputSerializationError)?;
    let sender_id = u64::from(recorded_note.note().metadata().sender()) as i64;
    let tag = u64::from(recorded_note.note().metadata().tag()) as i64;
    let num_assets = u64::from(recorded_note.note().metadata().num_assets()) as i64;
    let inclusion_proof = Some(
        serde_json::to_string(&recorded_note.proof())
            .map_err(StoreError::InputSerializationError)?,
    );
    let recipients = serde_json::to_string(&recorded_note.note().metadata().tag())
        .map_err(StoreError::InputSerializationError)?;
    let status = String::from("committed");
    let commit_height = recorded_note.origin().block_num;
    Ok((
        hash,
        nullifier,
        script,
        vault,
        inputs,
        serial_num,
        sender_id,
        tag,
        num_assets,
        inclusion_proof,
        recipients,
        status,
        commit_height,
    ))
}

/// Serialize the provided input note into database compatible types.
fn serialize_pending_note(note: &Note) -> Result<SerializedInputNoteData, StoreError> {
    let hash = serde_json::to_string(&note.hash()).map_err(StoreError::InputSerializationError)?;
    let nullifier = note.nullifier().inner().to_string();
    let script = note.script().to_bytes();
    let vault =
        serde_json::to_string(&note.vault()).map_err(StoreError::InputSerializationError)?;
    let inputs =
        serde_json::to_string(&note.inputs()).map_err(StoreError::InputSerializationError)?;
    let serial_num =
        serde_json::to_string(&note.serial_num()).map_err(StoreError::InputSerializationError)?;
    let sender_id = u64::from(note.metadata().sender()) as i64;
    let tag = u64::from(note.metadata().tag()) as i64;
    let num_assets = u64::from(note.metadata().num_assets()) as i64;
    let inclusion_proof = None;
    let recipients = serde_json::to_string(&note.metadata().tag())
        .map_err(StoreError::InputSerializationError)?;
    let status = String::from("pending");
    let commit_height = 0;

    Ok((
        hash,
        nullifier,
        script,
        vault,
        inputs,
        serial_num,
        sender_id,
        tag,
        num_assets,
        inclusion_proof,
        recipients,
        status,
        commit_height,
    ))
}

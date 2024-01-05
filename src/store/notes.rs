use crate::errors::StoreError;

use super::Store;

use clap::error::Result;
use crypto::utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable};

use objects::notes::{Note, NoteAssets, NoteId, NoteInclusionProof, NoteInputs, NoteScript};

use objects::transaction::InputNote;
use objects::{accounts::AccountId, notes::NoteMetadata, Digest, Felt};
use rusqlite::params;

pub(crate) const INSERT_NOTE_QUERY: &str = "\
INSERT INTO input_notes
    (note_id, nullifier, script, vault, inputs, serial_num, sender_id, tag, num_assets, inclusion_proof, recipients, status, commit_height)
 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

// TYPES
// ================================================================================================

type SerializedInputNoteData = (
    String,
    String,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
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
    Vec<u8>,
    Vec<u8>,
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

#[derive(Debug, PartialEq)]
pub struct InputNoteRecord {
    note: Note,
    inclusion_proof: Option<NoteInclusionProof>,
}

impl InputNoteRecord {
    pub const fn new(note: Note, inclusion_proof: Option<NoteInclusionProof>) -> InputNoteRecord {
        InputNoteRecord {
            note,
            inclusion_proof,
        }
    }
    pub const fn note(&self) -> &Note {
        &self.note
    }

    pub fn note_id(&self) -> NoteId {
        self.note.id()
    }

    pub  const fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        self.inclusion_proof.as_ref()
    }
}

impl Serializable for InputNoteRecord {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write(self.note().to_bytes());
        target.write(self.inclusion_proof.to_bytes());
    }
}

impl Deserializable for InputNoteRecord {
    fn read_from<R: ByteReader>(
        source: &mut R,
    ) -> std::prelude::v1::Result<Self, DeserializationError> {
        let note: Note = source.read()?;
        let proof: Option<NoteInclusionProof> = source.read()?;
        Ok(InputNoteRecord::new(note, proof))
    }
}

impl From<Note> for InputNoteRecord {
    fn from(note: Note) -> Self {
        InputNoteRecord {
            note,
            inclusion_proof: None,
        }
    }
}

impl From<InputNote> for InputNoteRecord {
    fn from(recorded_note: InputNote) -> Self {
        InputNoteRecord {
            note: recorded_note.note().clone(),
            inclusion_proof: Some(recorded_note.proof().clone()),
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
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
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
            .collect::<Result<Vec<InputNoteRecord>, _>>()
    }

    /// Retrieves the input note with the specified id from the database
    pub fn get_input_note_by_id(&self, note_id: NoteId) -> Result<InputNoteRecord, StoreError> {
        let query_id = &note_id.inner().to_string();
        const QUERY: &str = "SELECT script, inputs, vault, serial_num, sender_id, tag, num_assets, inclusion_proof FROM input_notes WHERE note_id = ?";

        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map(params![query_id.to_string()], parse_input_note_columns)
            .map_err(StoreError::QueryError)?
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_input_note)
            })
            .next()
            .ok_or(StoreError::InputNoteNotFound(note_id))?
    }

    /// Inserts the provided input note into the database
    pub fn insert_input_note(&self, note: &InputNoteRecord) -> Result<(), StoreError> {
        let (
            note_id,
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
        ) = serialize_input_note(note)?;

        self.db
            .execute(
                INSERT_NOTE_QUERY,
                params![
                    note_id,
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
    let inputs: Vec<u8> = row.get(1)?;
    let vault: Vec<u8> = row.get(2)?;
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
) -> Result<InputNoteRecord, StoreError> {
    let (script, inputs, note_assets, serial_num, sender_id, tag, num_assets, inclusion_proof) =
        serialized_input_note_parts;
    let script =
        NoteScript::read_from_bytes(&script).map_err(StoreError::DataDeserializationError)?;
    let inputs =
        NoteInputs::read_from_bytes(&inputs).map_err(StoreError::DataDeserializationError)?;
    let vault =
        NoteAssets::read_from_bytes(&note_assets).map_err(StoreError::DataDeserializationError)?;
    let serial_num =
        serde_json::from_str(&serial_num).map_err(StoreError::JsonDataDeserializationError)?;
    let note_metadata = NoteMetadata::new(
        AccountId::new_unchecked(Felt::new(sender_id)),
        Felt::new(tag),
        Felt::new(num_assets),
    );
    let note = Note::from_parts(script, inputs, vault, serial_num, note_metadata);

    let inclusion_proof = match inclusion_proof {
        Some(proof) => {
            let inclusion_proof =
                serde_json::from_str(&proof).map_err(StoreError::JsonDataDeserializationError)?;
            Some(inclusion_proof)
        }
        None => None,
    };

    Ok(InputNoteRecord::new(note, inclusion_proof))
}

/// Serialize the provided input note into database compatible types.
pub(crate) fn serialize_input_note(
    note: &InputNoteRecord,
) -> Result<SerializedInputNoteData, StoreError> {
    let note_id = note.note_id().inner().to_string();
    let nullifier = note.note().nullifier().inner().to_string();
    let script = note.note().script().to_bytes();
    let note_assets = note.note().assets().to_bytes();
    let inputs = note.note().inputs().to_bytes();
    let serial_num = serde_json::to_string(&note.note().serial_num())
        .map_err(StoreError::InputSerializationError)?;
    let sender_id = u64::from(note.note().metadata().sender()) as i64;
    let tag = u64::from(note.note().metadata().tag()) as i64;
    let num_assets = u64::from(note.note().metadata().num_assets()) as i64;
    let (inclusion_proof, commit_height, status) = match note.inclusion_proof() {
        Some(proof) => (
            Some(serde_json::to_string(&proof).map_err(StoreError::InputSerializationError)?),
            proof.origin().block_num,
            String::from("committed"),
        ),
        None => (None, 0u32, String::from("pending")),
    };

    let recipients = serde_json::to_string(&note.note().metadata().tag())
        .map_err(StoreError::InputSerializationError)?;
    Ok((
        note_id,
        nullifier,
        script,
        note_assets,
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

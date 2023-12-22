use crate::errors::StoreError;

use super::Store;

use clap::error::Result;
use crypto::utils::{Deserializable, Serializable};
use objects::notes::NoteScript;

use objects::{
    accounts::AccountId,
    notes::{Note, NoteMetadata, RecordedNote},
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
    String,
    String,
    String,
    i64,
);

type SerializedInputNoteParts = (Vec<u8>, String, String, String, u64, u64, u64, String);

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

impl Store {
    // NOTES
    // --------------------------------------------------------------------------------------------

    /// Retrieves the input notes from the database
    pub fn get_input_notes(
        &self,
        note_filter: InputNoteFilter,
    ) -> Result<Vec<RecordedNote>, StoreError> {
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
            .collect::<Result<Vec<RecordedNote>, _>>()
    }

    /// Retrieves the input note with the specified hash from the database
    pub fn get_input_note_by_hash(&self, hash: Digest) -> Result<RecordedNote, StoreError> {
        let query_hash =
            serde_json::to_string(&hash).map_err(StoreError::InputSerializationError)?;
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

    // STATE SYNC
    // --------------------------------------------------------------------------------------------

    /// Returns the note tags that the client is interested in.
    pub fn get_note_tags(&self) -> Result<Vec<u64>, StoreError> {
        const QUERY: &str = "SELECT tags FROM state_sync";

        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(|v: String| {
                        serde_json::from_str(&v).map_err(StoreError::JsonDataDeserializationError)
                    })
            })
            .next()
            .expect("state sync tags exist")
    }

    /// Adds a note tag to the list of tags that the client is interested in.
    pub fn add_note_tag(&mut self, tag: u64) -> Result<bool, StoreError> {
        let mut tags = self.get_note_tags()?;
        if tags.contains(&tag) {
            return Ok(false);
        }
        tags.push(tag);
        let tags = serde_json::to_string(&tags).map_err(StoreError::InputSerializationError)?;

        const QUERY: &str = "UPDATE state_sync SET tags = ?";
        self.db
            .execute(QUERY, params![tags])
            .map_err(StoreError::QueryError)
            .map(|_| ())?;

        Ok(true)
    }

    /// Returns the block number of the last state sync block
    pub fn get_latest_block_number(&self) -> Result<u32, StoreError> {
        const QUERY: &str = "SELECT block_number FROM state_sync";

        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .map(|v: i64| v as u32)
            })
            .next()
            .expect("state sync block number exists")
    }

    pub fn apply_state_sync(
        &mut self,
        block_number: u32,
        nullifiers: Vec<Digest>,
    ) -> Result<(), StoreError> {
        let tx = self
            .db
            .transaction()
            .map_err(StoreError::TransactionError)?;

        // update state sync block number
        const BLOCK_NUMBER_QUERY: &str = "UPDATE state_sync SET block_number = ?";
        tx.execute(BLOCK_NUMBER_QUERY, params![block_number])
            .map_err(StoreError::QueryError)?;

        // update spent notes
        for nullifier in nullifiers {
            const SPENT_QUERY: &str =
                "UPDATE input_notes SET status = 'consumed' WHERE nullifier = ?";
            let nullifier = nullifier.to_string();
            tx.execute(SPENT_QUERY, params![nullifier])
                .map_err(StoreError::QueryError)?;
        }

        // commit the updates
        tx.commit().map_err(StoreError::QueryError)?;

        Ok(())
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
    let inclusion_proof: String = row.get(7)?;
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
) -> Result<RecordedNote, StoreError> {
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

    let inclusion_proof =
        serde_json::from_str(&inclusion_proof).map_err(StoreError::JsonDataDeserializationError)?;
    Ok(RecordedNote::new(note, inclusion_proof))
}

/// Serialize the provided input note into database compatible types.
fn serialize_input_note(
    recorded_note: &RecordedNote,
) -> Result<SerializedInputNoteData, StoreError> {
    let hash = serde_json::to_string(&recorded_note.note().hash())
        .map_err(StoreError::InputSerializationError)?;
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
    let inclusion_proof = serde_json::to_string(&recorded_note.proof())
        .map_err(StoreError::InputSerializationError)?;
    let recipients = serde_json::to_string(&recorded_note.note().metadata().tag())
        .map_err(StoreError::InputSerializationError)?;
    let status = String::from("committed");
    let commit_height = recorded_note.origin().block_num as i64;
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

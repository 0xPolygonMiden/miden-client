use crate::errors::StoreError;

use super::{Store, NoteFilter};

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

/*
    hash BLOB NOT NULL,                                     -- the note hash
    vault BLOB NOT NULL,                                    -- the serialized NoteVault, including vault hash and list of assets
    recipient BLOB NOT NULL,                                -- serialized note recipient information (note script, note outputs, and serial_num).
    sender_id UNSIGNED BIG INT NOT NULL,                    -- the account ID of the sender
    tag UNSIGNED BIG INT NOT NULL,                          -- the note tag
    inclusion_proof BLOB NOT NULL,                          -- the inclusion proof of the note against a block number
    recipients BLOB NOT NULL,                               -- a list of account IDs of accounts which can consume this note
    status TEXT CHECK( status IN (                          -- the status of the note - either pending, committed or consumed
        'pending', 'committed', 'consumed'
        )),
    commit_height UNSIGNED BIG INT NOT NULL,                -- the block number at which the note was included into the chain
*/

type SerializedOutputNoteData = (
    String,
    String,
    String,
    i64,
    String,
    String,
    String,
    i64,
);

type SerializedInputNoteParts = (Vec<u8>, String, String, String, u64, u64, u64, String);

impl Store {
    // NOTES
    // --------------------------------------------------------------------------------------------

    /// Retrieves the output notes from the database
    pub fn get_output_notes(
        &self,
        note_filter: NoteFilter,
    ) -> Result<Vec<RecordedNote>, StoreError> {
        self.db
            .prepare(&note_filter.to_query())
            .map_err(StoreError::QueryError)?
            .query_map([], parse_output_note_columns)
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_output_note)
            })
            .collect::<Result<Vec<RecordedNote>, _>>()
    }

    /// Retrieves the output note with the specified hash from the database
    pub fn get_output_note_by_hash(&self, hash: Digest) -> Result<RecordedNote, StoreError> {
        let query_hash =
            serde_json::to_string(&hash).map_err(StoreError::InputSerializationError)?;
        const QUERY: &str = "SELECT script, outputs, vault, serial_num, sender_id, tag, num_assets, inclusion_proof FROM output_notes WHERE hash = ?";

        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map(params![query_hash.to_string()], parse_output_note_columns)
            .map_err(StoreError::QueryError)?
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_output_note)
            })
            .next()
            .ok_or(StoreError::InputNoteNotFound(hash))?
    }

    /// Inserts the provided output note into the database
    pub fn insert_output_note(&self, recorded_note: &RecordedNote) -> Result<(), StoreError> {
        let (
            hash,
            nullifier,
            script,
            vault,
            outputs,
            serial_num,
            sender_id,
            tag,
            num_assets,
            inclusion_proof,
            recipients,
            status,
            commit_height,
        ) = serialize_output_note(recorded_note)?;

        const QUERY: &str = "\
        INSERT INTO output_notes
            (hash, nullifier, script, vault, outputs, serial_num, sender_id, tag, num_assets, inclusion_proof, recipients, status, commit_height)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

        self.db
            .execute(
                QUERY,
                params![
                    hash,
                    nullifier,
                    script,
                    vault,
                    outputs,
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

    /// Returns the nullifiers of all unspent output notes
    pub fn get_unspent_output_note_nullifiers(&self) -> Result<Vec<Digest>, StoreError> {
        const QUERY: &str = "SELECT nullifier FROM output_notes WHERE status = 'committed'";

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

/// Parse output note columns from the provided row into native types.
fn parse_output_note_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedInputNoteParts, rusqlite::Error> {
    let script: Vec<u8> = row.get(0)?;
    let outputs: String = row.get(1)?;
    let vault: String = row.get(2)?;
    let serial_num: String = row.get(3)?;
    let sender_id = row.get::<usize, i64>(4)? as u64;
    let tag = row.get::<usize, i64>(5)? as u64;
    let num_assets = row.get::<usize, i64>(6)? as u64;
    let inclusion_proof: String = row.get(7)?;
    Ok((
        script,
        outputs,
        vault,
        serial_num,
        sender_id,
        tag,
        num_assets,
        inclusion_proof,
    ))
}

/// Parse a note from the provided parts.
fn parse_output_note(
    serialized_output_note_parts: SerializedInputNoteParts,
) -> Result<RecordedNote, StoreError> {
    let (script, outputs, vault, serial_num, sender_id, tag, num_assets, inclusion_proof) =
        serialized_output_note_parts;
    let script =
        NoteScript::read_from_bytes(&script).map_err(StoreError::DataDeserializationError)?;
    let outputs = serde_json::from_str(&outputs).map_err(StoreError::JsonDataDeserializationError)?;
    let vault = serde_json::from_str(&vault).map_err(StoreError::JsonDataDeserializationError)?;
    let serial_num =
        serde_json::from_str(&serial_num).map_err(StoreError::JsonDataDeserializationError)?;
    let note_metadata = NoteMetadata::new(
        AccountId::new_unchecked(Felt::new(sender_id)),
        Felt::new(tag),
        Felt::new(num_assets),
    );
    let note = Note::from_parts(script, outputs, vault, serial_num, note_metadata);

    let inclusion_proof =
        serde_json::from_str(&inclusion_proof).map_err(StoreError::JsonDataDeserializationError)?;
    Ok(RecordedNote::new(note, inclusion_proof))
}

/// Serialize the provided output note into database compatible types.
fn serialize_output_note(
    recorded_note: &RecordedNote,
) -> Result<SerializedOutputNoteData, StoreError> {
    let hash = serde_json::to_string(&recorded_note.note().hash())
        .map_err(StoreError::InputSerializationError)?;
    let nullifier = recorded_note.note().nullifier().inner().to_string();
    let script = recorded_note.note().script().to_bytes();
    let vault = serde_json::to_string(&recorded_note.note().vault())
        .map_err(StoreError::InputSerializationError)?;
    let outputs = serde_json::to_string(&recorded_note.note().outputs())
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
    let commit_height = recorded_note.origin().block_num.inner() as i64;
    Ok((
        hash,
        nullifier,
        script,
        vault,
        outputs,
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

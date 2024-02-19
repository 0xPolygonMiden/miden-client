use std::fmt;

use crate::errors::StoreError;
use crate::store::{InputNoteRecord, NoteFilter};

use super::SqliteStore;

use clap::error::Result;

use crypto::utils::{Deserializable, Serializable};

use objects::notes::{Note, NoteAssets, NoteId, NoteInclusionProof, NoteInputs, NoteScript};

use objects::{accounts::AccountId, notes::NoteMetadata, Felt};
use rusqlite::{params, Transaction};

fn insert_note_query(table_name: NoteTable) -> String {
    format!("\
    INSERT INTO {table_name}
        (note_id, nullifier, script, assets, inputs, serial_num, sender_id, tag, inclusion_proof, recipient, status)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
}

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
    Option<Vec<u8>>,
    String,
    String,
);

type SerializedInputNoteParts = (Vec<u8>, Vec<u8>, Vec<u8>, String, u64, u64, Option<Vec<u8>>);

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
    fn to_query(&self) -> String {
        let base = String::from("SELECT script, inputs, assets, serial_num, sender_id, tag, inclusion_proof FROM input_notes");
        match self {
            NoteFilter::All => base,
            NoteFilter::Committed => format!("{base} WHERE status = 'committed'"),
            NoteFilter::Consumed => format!("{base} WHERE status = 'consumed'"),
            NoteFilter::Pending => format!("{base} WHERE status = 'pending'"),
        }
    }
}

// NOTES STORE METHODS
// --------------------------------------------------------------------------------------------

impl SqliteStore {
    pub(crate) fn get_input_notes(
        &self,
        note_filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.db
            .prepare(&note_filter.to_query())?
            .query_map([], parse_input_note_columns)
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).and_then(parse_input_note))
            .collect::<Result<Vec<InputNoteRecord>, _>>()
    }

    pub(crate) fn get_input_note_by_id(
        &self,
        note_id: NoteId,
    ) -> Result<InputNoteRecord, StoreError> {
        let query_id = &note_id.inner().to_string();
        const QUERY: &str = "SELECT script, inputs, assets, serial_num, sender_id, tag, inclusion_proof FROM input_notes WHERE note_id = ?";

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
        nullifier,
        script,
        vault,
        inputs,
        serial_num,
        sender_id,
        tag,
        inclusion_proof,
        recipient,
        status,
    ) = serialize_note(note)?;

    tx.execute(
        &insert_note_query(NoteTable::InputNotes),
        params![
            note_id,
            nullifier,
            script,
            vault,
            inputs,
            serial_num,
            sender_id,
            tag,
            inclusion_proof,
            recipient,
            status,
        ],
    )
    .map_err(|err| StoreError::QueryError(err.to_string()))
    .map(|_| ())
}

/// Inserts the provided input note into the database
pub fn insert_output_note_tx(
    tx: &Transaction<'_>,
    note: &InputNoteRecord,
) -> Result<(), StoreError> {
    let (
        note_id,
        nullifier,
        script,
        vault,
        inputs,
        serial_num,
        sender_id,
        tag,
        inclusion_proof,
        recipient,
        status,
    ) = serialize_note(note)?;

    tx.execute(
        &insert_note_query(NoteTable::OutputNotes),
        params![
            note_id,
            nullifier,
            script,
            vault,
            inputs,
            serial_num,
            sender_id,
            tag,
            inclusion_proof,
            recipient,
            status,
        ],
    )
    .map_err(|err| StoreError::QueryError(err.to_string()))
    .map(|_| ())
}

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
    let inclusion_proof: Option<Vec<u8>> = row.get(6)?;
    Ok((
        script,
        inputs,
        vault,
        serial_num,
        sender_id,
        tag,
        inclusion_proof,
    ))
}

/// Parse a note from the provided parts.
fn parse_input_note(
    serialized_input_note_parts: SerializedInputNoteParts,
) -> Result<InputNoteRecord, StoreError> {
    let (script, inputs, note_assets, serial_num, sender_id, tag, inclusion_proof) =
        serialized_input_note_parts;
    let script = NoteScript::read_from_bytes(&script)?;
    let inputs = NoteInputs::read_from_bytes(&inputs)?;
    let vault = NoteAssets::read_from_bytes(&note_assets)?;
    let serial_num =
        serde_json::from_str(&serial_num).map_err(StoreError::JsonDataDeserializationError)?;
    let note_metadata = NoteMetadata::new(
        AccountId::new_unchecked(Felt::new(sender_id)),
        Felt::new(tag),
    );
    let note = Note::from_parts(script, inputs, vault, serial_num, note_metadata);

    let inclusion_proof = inclusion_proof
        .map(|proof| NoteInclusionProof::read_from_bytes(&proof))
        .transpose()?;

    Ok(InputNoteRecord::new(note, inclusion_proof))
}

/// Serialize the provided input note into database compatible types.
pub(crate) fn serialize_note(
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
    let (inclusion_proof, status) = match note.inclusion_proof() {
        Some(proof) => {
            // FIXME: This removal is to accomodate a problem with how the node constructs paths where
            // they are constructed using note ID instead of authentication hash, so for now we remove the first
            // node here.
            //
            // See: https://github.com/0xPolygonMiden/miden-node/blob/main/store/src/state.rs#L274
            let mut path = proof.note_path().clone();
            if path.len() > 0 {
                let _removed = path.remove(0);
            }

            (
                Some(
                    NoteInclusionProof::new(
                        proof.origin().block_num,
                        proof.sub_hash(),
                        proof.note_root(),
                        proof.origin().node_index.value(),
                        path,
                    )
                    .map_err(StoreError::NoteInclusionProofError)?
                    .to_bytes(),
                ),
                String::from("committed"),
            )
        }
        None => (None, String::from("pending")),
    };
    let recipient = note.note().recipient().to_hex();

    Ok((
        note_id,
        nullifier,
        script,
        note_assets,
        inputs,
        serial_num,
        sender_id,
        tag,
        inclusion_proof,
        recipient,
        status,
    ))
}

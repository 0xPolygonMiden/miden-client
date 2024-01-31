use core::fmt;

use crate::errors::{ClientError, StoreError};

use super::Store;

use clap::error::Result;

use crypto::utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable};

use objects::notes::{Note, NoteAssets, NoteId, NoteInclusionProof, NoteInputs, NoteScript};

use objects::{accounts::AccountId, notes::NoteMetadata, transaction::InputNote, Digest, Felt};
use rusqlite::{named_params, params, Transaction};

fn insert_note_query(table_name: NoteTable) -> String {
    dbg!(format!("\
    INSERT INTO {table_name}
        (note_id, nullifier, script, assets, inputs, serial_num, inclusion_proof, recipient, status, metadata)
     VALUES (:note_id, :nullifier, :script, :assets, :inputs, :serial_num, :inclusion_proof, :recipient, :status, json(:somedata))"))
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
    Option<Vec<u8>>,
    String,
    String,
    String,
);

type SerializedInputNoteParts = (Vec<u8>, Vec<u8>, Vec<u8>, String, u64, u64, Option<Vec<u8>>);

// NOTE TABLE
// ================================================================================================
/// Represents a table in the db used to store notes based on their use case
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
/// Represents a filter for input notes
pub enum NoteFilter {
    All,
    Consumed,
    Committed,
    Pending,
}

impl NoteFilter {
    fn to_query(&self, notes_table: NoteTable) -> String {
        let base = format!("SELECT script, inputs, assets, serial_num, json_extract(metadata, '$.sender_id'), json_extract(metadata, '$.tag'), inclusion_proof FROM {notes_table}");
        match self {
            NoteFilter::All => base,
            NoteFilter::Committed => format!("{base} WHERE status = 'committed'"),
            NoteFilter::Consumed => format!("{base} WHERE status = 'consumed'"),
            NoteFilter::Pending => format!("{base} WHERE status = 'pending'"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InputNoteRecord {
    note: Note,
    inclusion_proof: Option<NoteInclusionProof>,
}

impl InputNoteRecord {
    pub fn new(note: Note, inclusion_proof: Option<NoteInclusionProof>) -> InputNoteRecord {
        InputNoteRecord {
            note,
            inclusion_proof,
        }
    }
    pub fn note(&self) -> &Note {
        &self.note
    }

    pub fn note_id(&self) -> NoteId {
        self.note.id()
    }

    pub fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
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

impl TryInto<InputNote> for InputNoteRecord {
    type Error = ClientError;

    fn try_into(self) -> Result<InputNote, Self::Error> {
        match self.inclusion_proof() {
            Some(proof) => Ok(InputNote::new(self.note().clone(), proof.clone())),
            None => Err(ClientError::NoteError(
                objects::NoteError::invalid_origin_index(
                    "Input Note Record contains no proof".to_string(),
                ),
            )),
        }
    }
}

// NOTES STORE METHODS
// --------------------------------------------------------------------------------------------

impl Store {
    /// Retrieves the input notes from the database
    pub fn get_input_notes(
        &self,
        note_filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.db
            .prepare(&note_filter.to_query(NoteTable::InputNotes))?
            .query_map([], parse_input_note_columns)
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).and_then(parse_input_note))
            .collect::<Result<Vec<InputNoteRecord>, _>>()
    }

    /// Retrieves the output notes from the database
    pub fn get_output_notes(
        &self,
        note_filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.db
            .prepare(&note_filter.to_query(NoteTable::OutputNotes))?
            .query_map([], parse_input_note_columns)
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).and_then(parse_input_note))
            .collect::<Result<Vec<InputNoteRecord>, _>>()
    }

    /// Retrieves the input note with the specified id from the database
    pub fn get_input_note_by_id(&self, note_id: NoteId) -> Result<InputNoteRecord, StoreError> {
        let query_id = &note_id.inner().to_string();
        const QUERY: &str = "SELECT script, inputs, assets, serial_num, json_extract(metadata, '$.sender_id'), json_extract(metadata, '$.tag'), inclusion_proof FROM input_notes WHERE note_id = ?";

        self.db
            .prepare(QUERY)?
            .query_map(params![query_id.to_string()], parse_input_note_columns)?
            .map(|result| Ok(result?).and_then(parse_input_note))
            .next()
            .ok_or(StoreError::InputNoteNotFound(note_id))?
    }

    /// Inserts the provided input note into the database
    pub fn insert_input_note(&mut self, note: &InputNoteRecord) -> Result<(), StoreError> {
        let tx = self.db.transaction()?;

        Self::insert_input_note_tx(&tx, note)?;

        Ok(tx.commit()?)
    }

    /// Returns the nullifiers of all unspent input notes
    pub fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Digest>, StoreError> {
        const QUERY: &str = "SELECT nullifier FROM input_notes WHERE status = 'committed'";

        self.db
            .prepare(QUERY)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(|err| StoreError::ParsingError(err.to_string()))
                    .and_then(|v: String| Digest::try_from(v).map_err(StoreError::HexParseError))
            })
            .collect::<Result<Vec<Digest>, _>>()
    }

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
            inclusion_proof,
            recipient,
            status,
            metadata,
        ) = serialize_input_note(note)?;

        tx.execute(
            &dbg!(insert_note_query(NoteTable::InputNotes)),
            named_params! {
                ":note_id": note_id,
                ":nullifier": nullifier,
                ":script": script,
                ":assets": vault,
                ":inputs": inputs,
                ":serial_num": serial_num,
                ":inclusion_proof": inclusion_proof,
                ":recipient": recipient,
                ":status": status,
                ":somedata": metadata,
            },
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
            inclusion_proof,
            recipient,
            status,
            metadata,
        ) = serialize_input_note(note)?;

        tx.execute(
            &dbg!(insert_note_query(NoteTable::OutputNotes)),
            named_params! {
                ":note_id": note_id,
                ":nullifier": nullifier,
                ":script": script,
                ":assets": vault,
                ":inputs": inputs,
                ":serial_num": serial_num,
                ":inclusion_proof": inclusion_proof,
                ":recipient": recipient,
                ":status": status,
                ":somedata": metadata,
            },
        )
        .map_err(|err| StoreError::QueryError(err.to_string()))
        .map(|_| ())
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
                    .unwrap()
                    .to_bytes(),
                ),
                String::from("committed"),
            )
        }
        None => (None, String::from("pending")),
    };
    let recipient = note.note().recipient().to_hex();

    let metadata = format!(r#"{{"sender_id": {sender_id}, "tag": {tag}}}"#);

    dbg!(&metadata);

    Ok((
        note_id,
        nullifier,
        script,
        note_assets,
        inputs,
        serial_num,
        inclusion_proof,
        recipient,
        status,
        metadata,
    ))
}

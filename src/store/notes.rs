use core::fmt;

use crate::errors::{ClientError, StoreError};

use super::Store;

use clap::error::Result;

use crypto::merkle::MerklePath;
use crypto::utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable};
use crypto::Word;

use objects::notes::{Note, NoteAssets, NoteId, NoteInclusionProof, NoteInputs, NoteScript};

use objects::{accounts::AccountId, notes::NoteMetadata, transaction::InputNote, Digest, Felt};
use rusqlite::{named_params, params, Transaction};
use serde::{Deserialize, Serialize};

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
    String,
    String,
    Option<String>,
);

type SerializedInputNoteParts = (Vec<u8>, String, String, Option<String>);

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
#[derive(Clone, Debug)]
pub enum NoteFilter {
    All,
    Consumed,
    Committed,
    Pending,
}

impl NoteFilter {
    /// Returns a [String] containing the query for this Filter
    fn to_query(&self, notes_table: NoteTable) -> String {
        let base = format!(
            "SELECT 
                    assets, 
                    details, 
                    metadata,
                    inclusion_proof
                    from {notes_table}"
        );

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
        self.note().write_into(target);
        self.inclusion_proof.write_into(target);
    }
}

impl Deserializable for InputNoteRecord {
    fn read_from<R: ByteReader>(
        source: &mut R,
    ) -> std::prelude::v1::Result<Self, DeserializationError> {
        let note = Note::read_from(source)?;
        let proof = Option::<NoteInclusionProof>::read_from(source)?;
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

// TODO: Once SqliteStore change is implemented move it to that module
#[derive(Serialize, Deserialize)]
struct NoteRecordMetadata {
    sender_id: AccountId,
    tag: Felt,
}

impl NoteRecordMetadata {
    fn new(sender_id: AccountId, tag: Felt) -> Self {
        Self { sender_id, tag }
    }

    fn sender_id(&self) -> &AccountId {
        &self.sender_id
    }

    fn tag(&self) -> &Felt {
        &self.tag
    }
}

#[derive(Serialize, Deserialize)]
struct NoteRecordDetails {
    nullifier: String,
    script: Vec<u8>,
    inputs: Vec<u8>,
    serial_num: Word,
}

impl NoteRecordDetails {
    fn new(nullifier: String, script: Vec<u8>, inputs: Vec<u8>, serial_num: Word) -> Self {
        Self {
            nullifier,
            script,
            inputs,
            serial_num,
        }
    }

    fn script(&self) -> &Vec<u8> {
        &self.script
    }

    fn inputs(&self) -> &Vec<u8> {
        &self.inputs
    }

    fn serial_num(&self) -> &Word {
        &self.serial_num
    }
}

#[derive(Serialize, Deserialize)]
struct NoteRecordInclusionProof {
    block_num: u32,
    note_index: u64,
    sub_hash: String,
    note_root: String,
    note_path: Vec<String>,
}

impl NoteRecordInclusionProof {
    fn new(
        block_num: u32,
        note_index: u64,
        sub_hash: String,
        note_root: String,
        note_path: Vec<String>,
    ) -> Self {
        Self {
            block_num,
            note_index,
            sub_hash,
            note_root,
            note_path,
        }
    }

    fn block_num(&self) -> u32 {
        self.block_num
    }

    fn note_index(&self) -> u64 {
        self.note_index
    }

    fn sub_hash(&self) -> &str {
        &self.sub_hash
    }

    fn note_root(&self) -> &str {
        &self.note_root
    }

    fn note_path(&self) -> &Vec<String> {
        &self.note_path
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

        const QUERY: &str = "SELECT 
                                    assets, 
                                    details,
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

    /// Inserts the provided input note into the database
    pub fn insert_input_note(&mut self, note: &InputNoteRecord) -> Result<(), StoreError> {
        let tx = self.db.transaction()?;

        Self::insert_input_note_tx(&tx, note)?;

        Ok(tx.commit()?)
    }

    /// Returns the nullifiers of all unspent input notes
    pub fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Digest>, StoreError> {
        const QUERY: &str = "SELECT json_extract(details, '$.nullifier') FROM input_notes WHERE status = 'committed'";

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
        note: &InputNoteRecord,
    ) -> Result<(), StoreError> {
        let (note_id, assets, recipient, status, metadata, details, inclusion_proof) =
            serialize_input_note(note)?;

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
}

// HELPERS
// ================================================================================================

/// Parse input note columns from the provided row into native types.
fn parse_input_note_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedInputNoteParts, rusqlite::Error> {
    let assets: Vec<u8> = row.get(0)?;
    let details: String = row.get(1)?;
    let metadata: String = row.get(2)?;
    let inclusion_proof: Option<String> = row.get(3)?;

    Ok((assets, details, metadata, inclusion_proof))
}

/// Parse a note from the provided parts.
fn parse_input_note(
    serialized_input_note_parts: SerializedInputNoteParts,
) -> Result<InputNoteRecord, StoreError> {
    let (note_assets, note_details, note_metadata, note_inclusion_proof) =
        serialized_input_note_parts;

    let note_details: NoteRecordDetails =
        serde_json::from_str(&note_details).map_err(StoreError::JsonDataDeserializationError)?;
    let note_metadata: NoteRecordMetadata =
        serde_json::from_str(&note_metadata).map_err(StoreError::JsonDataDeserializationError)?;

    let script = NoteScript::read_from_bytes(note_details.script())?;
    let inputs = NoteInputs::read_from_bytes(note_details.inputs())?;

    let serial_num = note_details.serial_num();
    let note_metadata = NoteMetadata::new(*note_metadata.sender_id(), *note_metadata.tag());
    let note_assets = NoteAssets::read_from_bytes(&note_assets)?;
    let note = Note::from_parts(script, inputs, note_assets, *serial_num, note_metadata);

    let inclusion_proof = match note_inclusion_proof {
        Some(note_inclusion_proof) => {
            let note_inclusion_proof: NoteRecordInclusionProof =
                serde_json::from_str(&note_inclusion_proof)
                    .map_err(StoreError::JsonDataDeserializationError)?;

            let sub_hash = Digest::try_from(note_inclusion_proof.sub_hash())?;
            let note_root = Digest::try_from(note_inclusion_proof.note_root())?;
            let note_path = note_inclusion_proof
                .note_path()
                .iter()
                .map(Digest::try_from)
                .collect::<Result<Vec<_>, _>>()?;

            let note_path = MerklePath::from(note_path);
            Some(
                NoteInclusionProof::new(
                    note_inclusion_proof.block_num(),
                    sub_hash,
                    note_root,
                    note_inclusion_proof.note_index(),
                    note_path,
                )
                .expect("Should be able to read note inclusion proof from db"),
            )
        }
        _ => None,
    };

    Ok(InputNoteRecord::new(note, inclusion_proof))
}

/// Serialize the provided input note into database compatible types.
pub(crate) fn serialize_input_note(
    note: &InputNoteRecord,
) -> Result<SerializedInputNoteData, StoreError> {
    let note_id = note.note_id().inner().to_string();
    let note_assets = note.note().assets().to_bytes();
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

            let block_num = proof.origin().block_num;
            let node_index = proof.origin().node_index.value();
            let sub_hash = proof.sub_hash().to_string();
            let note_root = proof.note_root().to_string();
            let path = path
                .into_iter()
                .map(|path_node| path_node.to_string())
                .collect::<Vec<_>>();

            let inclusion_proof = serde_json::to_string(&NoteRecordInclusionProof::new(
                block_num, node_index, sub_hash, note_root, path,
            ))
            .map_err(StoreError::InputSerializationError)?;

            (Some(inclusion_proof), String::from("committed"))
        }
        None => (None, String::from("pending")),
    };
    let recipient = note.note().recipient().to_hex();

    let sender_id = note.note().metadata().sender();
    let tag = note.note().metadata().tag();
    let metadata = serde_json::to_string(&NoteRecordMetadata::new(sender_id, tag))
        .map_err(StoreError::InputSerializationError)?;

    let nullifier = note.note().nullifier().inner().to_string();
    let script = note.note().script().to_bytes();
    let inputs = note.note().inputs().to_bytes();
    let serial_num = note.note().serial_num();
    let details = serde_json::to_string(&NoteRecordDetails::new(
        nullifier, script, inputs, serial_num,
    ))
    .map_err(StoreError::InputSerializationError)?;

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

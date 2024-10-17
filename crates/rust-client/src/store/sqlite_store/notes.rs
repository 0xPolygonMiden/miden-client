use alloc::{
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    crypto::utils::{Deserializable, Serializable},
    notes::{
        NoteAssets, NoteDetails, NoteInputs, NoteMetadata, NoteRecipient, NoteScript, Nullifier,
    },
    Digest, Word,
};
use rusqlite::{named_params, params, params_from_iter, types::Value, Transaction};

use super::SqliteStore;
use crate::store::{
    note_record::OutputNoteState, InputNoteRecord, InputNoteState, NoteFilter, OutputNoteRecord,
    StoreError,
};

// TYPES
// ================================================================================================

/// Represents an `InputNoteRecord` serialized to be stored in the database
struct SerializedInputNoteData {
    pub id: String,
    pub assets: Vec<u8>,
    pub serial_number: Vec<u8>,
    pub inputs: Vec<u8>,
    pub script_hash: String,
    pub script: Vec<u8>,
    pub nullifier: String,
    pub state_discriminant: u8,
    pub state: Vec<u8>,
}

/// Represents an `OutputNoteRecord` serialized to be stored in the database
struct SerializedOutputNoteData {
    pub id: String,
    pub assets: Vec<u8>,
    pub metadata: Vec<u8>,
    pub nullifier: Option<String>,
    pub recipient_digest: String,
    pub expected_height: u32,
    pub state_discriminant: u8,
    pub state: Vec<u8>,
}

/// Represents the parts retrieved from the database to build an `InputNoteRecord`
struct SerializedInputNoteParts {
    pub assets: Vec<u8>,
    pub serial_number: Vec<u8>,
    pub inputs: Vec<u8>,
    pub script: Vec<u8>,
    pub state: Vec<u8>,
    pub created_at: u64,
}

/// Represents the parts retrieved from the database to build an `OutputNoteRecord`
struct SerializedOutputNoteParts {
    pub assets: Vec<u8>,
    pub metadata: Vec<u8>,
    pub recipient_digest: String,
    pub expected_height: u32,
    pub state: Vec<u8>,
}

// NOTE FILTER
// ================================================================================================
type NoteQueryParams = Vec<Rc<Vec<Value>>>;
impl NoteFilter {
    /// Returns a [String] containing the query for this Filter
    fn to_query_output_notes(&self) -> (String, NoteQueryParams) {
        let base = "SELECT
                    note.recipient_digest,
                    note.assets,
                    note.metadata,
                    note.expected_height,
                    note.state
                    from output_notes AS note";

        let (condition, params) = self.output_notes_condition();
        let query = format!("{base} WHERE {condition}");

        (query, params)
    }

    fn output_notes_condition(&self) -> (String, NoteQueryParams) {
        let mut params = Vec::new();
        let condition = match self {
            NoteFilter::All => "1 = 1".to_string(),
            NoteFilter::Committed => {
                format!(
                    "state_discriminant in ({}, {})",
                    OutputNoteState::STATE_COMMITTED_PARTIAL,
                    OutputNoteState::STATE_COMMITTED_FULL
                )
            },
            NoteFilter::Consumed => {
                format!("state_discriminant = {}", OutputNoteState::STATE_CONSUMED)
            },
            NoteFilter::Expected => {
                format!(
                    "state_discriminant in ({}, {})",
                    OutputNoteState::STATE_EXPECTED_PARTIAL,
                    OutputNoteState::STATE_EXPECTED_FULL
                )
            },
            NoteFilter::Processing => "1 = 0".to_string(), // There are no processing output notes
            NoteFilter::Unique(note_id) => {
                let note_ids_list = vec![Value::Text(note_id.inner().to_string())];
                params.push(Rc::new(note_ids_list));
                "note.note_id IN rarray(?)".to_string()
            },
            NoteFilter::List(note_ids) => {
                let note_ids_list = note_ids
                    .iter()
                    .map(|note_id| Value::Text(note_id.inner().to_string()))
                    .collect::<Vec<Value>>();

                params.push(Rc::new(note_ids_list));
                "note.note_id IN rarray(?)".to_string()
            },
            NoteFilter::Nullifiers(nullifiers) => {
                let nullifiers_list = nullifiers
                    .iter()
                    .map(|nullifier| Value::Text(nullifier.to_string()))
                    .collect::<Vec<Value>>();

                params.push(Rc::new(nullifiers_list));
                "note.nullifier IN rarray(?)".to_string()
            },
            NoteFilter::Unverified => "1 = 0".to_string(), // there are no unverified output notes
            NoteFilter::Unspent => {
                format!(
                    "state_discriminant in ({}, {})",
                    OutputNoteState::STATE_EXPECTED_FULL,
                    OutputNoteState::STATE_COMMITTED_FULL,
                )
            },
        };

        (condition, params)
    }

    fn to_query_input_notes(&self) -> (String, NoteQueryParams) {
        let base = "SELECT
                note.assets,
                note.serial_number,
                note.inputs,
                script.serialized_note_script,
                note.state,
                note.created_at
                from input_notes AS note
                LEFT OUTER JOIN notes_scripts AS script
                    ON note.script_hash = script.script_hash";

        let (condition, params) = self.input_notes_condition();
        let query = format!("{base} WHERE {condition}");

        (query, params)
    }

    fn input_notes_condition(&self) -> (String, NoteQueryParams) {
        let mut params = Vec::new();
        let condition = match self {
            NoteFilter::All => "(1 = 1)".to_string(),
            NoteFilter::Committed => {
                format!("(state_discriminant = {})", InputNoteState::STATE_COMMITTED)
            },
            NoteFilter::Consumed => {
                format!(
                    "(state_discriminant in ({}, {}, {}))",
                    InputNoteState::STATE_CONSUMED_AUTHENTICATED_LOCAL,
                    InputNoteState::STATE_CONSUMED_UNAUTHENTICATED_LOCAL,
                    InputNoteState::STATE_CONSUMED_EXTERNAL
                )
            },
            NoteFilter::Expected => {
                format!("(state_discriminant = {})", InputNoteState::STATE_EXPECTED)
            },
            NoteFilter::Processing => {
                format!(
                    "(state_discriminant in ({}, {}))",
                    InputNoteState::STATE_PROCESSING_AUTHENTICATED,
                    InputNoteState::STATE_PROCESSING_UNAUTHENTICATED
                )
            },
            NoteFilter::Unique(note_id) => {
                let note_ids_list = vec![Value::Text(note_id.inner().to_string())];
                params.push(Rc::new(note_ids_list));
                "(note.note_id IN rarray(?))".to_string()
            },
            NoteFilter::List(note_ids) => {
                let note_ids_list = note_ids
                    .iter()
                    .map(|note_id| Value::Text(note_id.inner().to_string()))
                    .collect::<Vec<Value>>();

                params.push(Rc::new(note_ids_list));
                "(note.note_id IN rarray(?))".to_string()
            },
            NoteFilter::Nullifiers(nullifiers) => {
                let nullifiers_list = nullifiers
                    .iter()
                    .map(|nullifier| Value::Text(nullifier.to_string()))
                    .collect::<Vec<Value>>();

                params.push(Rc::new(nullifiers_list));
                "(note.nullifier IN rarray(?))".to_string()
            },
            NoteFilter::Unverified => {
                format!("(state_discriminant = {})", InputNoteState::STATE_UNVERIFIED)
            },
            NoteFilter::Unspent => {
                format!(
                    "(state_discriminant in ({}, {}, {}, {}, {}))",
                    InputNoteState::STATE_EXPECTED,
                    InputNoteState::STATE_PROCESSING_AUTHENTICATED,
                    InputNoteState::STATE_PROCESSING_UNAUTHENTICATED,
                    InputNoteState::STATE_UNVERIFIED,
                    InputNoteState::STATE_COMMITTED
                )
            },
        };

        (condition, params)
    }
}

// NOTES STORE METHODS
// --------------------------------------------------------------------------------------------

impl SqliteStore {
    pub(crate) fn get_input_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        let (query, params) = filter.to_query_input_notes();
        let notes = self
            .db()
            .prepare(query.as_str())?
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
        let (query, params) = filter.to_query_output_notes();

        let notes = self
            .db()
            .prepare(&query)?
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

    pub(crate) fn upsert_input_notes(&self, notes: &[InputNoteRecord]) -> Result<(), StoreError> {
        let mut db = self.db();
        let tx = db.transaction()?;

        for note in notes {
            upsert_input_note_tx(&tx, note)?;
        }

        Ok(tx.commit()?)
    }

    pub(crate) fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        const QUERY: &str =
            "SELECT nullifier FROM input_notes WHERE state_discriminant NOT IN rarray(?)";
        let unspent_filters = Rc::new(vec![
            Value::from(InputNoteState::STATE_CONSUMED_AUTHENTICATED_LOCAL.to_string()),
            Value::from(InputNoteState::STATE_CONSUMED_UNAUTHENTICATED_LOCAL.to_string()),
            Value::from(InputNoteState::STATE_CONSUMED_EXTERNAL.to_string()),
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
}

// HELPERS
// ================================================================================================

/// Inserts the provided input note into the database, if the note already exists, it will be
/// replaced.
pub(super) fn upsert_input_note_tx(
    tx: &Transaction<'_>,
    note: &InputNoteRecord,
) -> Result<(), StoreError> {
    let SerializedInputNoteData {
        id,
        assets,
        serial_number,
        inputs,
        script_hash,
        script,
        nullifier,
        state_discriminant,
        state,
    } = serialize_input_note(note)?;

    const SCRIPT_QUERY: &str =
        "INSERT OR REPLACE INTO notes_scripts (script_hash, serialized_note_script) VALUES (?, ?)";
    tx.execute(SCRIPT_QUERY, params![script_hash, script,])?;

    const NOTE_QUERY: &str = "
        INSERT OR REPLACE INTO input_notes (
            note_id,
            assets,
            serial_number,
            inputs,
            script_hash,
            nullifier,
            state_discriminant,
            state,
            created_at
        ) VALUES (
            :note_id,
            :assets,
            :serial_number,
            :inputs,
            :script_hash,
            :nullifier,
            :state_discriminant,
            :state,
            unixepoch(current_timestamp));
    ";

    tx.execute(
        NOTE_QUERY,
        named_params! {
            ":note_id": id,
            ":assets": assets,
            ":serial_number": serial_number,
            ":inputs": inputs,
            ":script_hash": script_hash,
            ":nullifier": nullifier,
            ":state_discriminant": state_discriminant,
            ":state": state,
        },
    )
    .map_err(|err| StoreError::QueryError(err.to_string()))
    .map(|_| ())
}

/// Inserts the provided input note into the database
pub fn upsert_output_note_tx(
    tx: &Transaction<'_>,
    note: &OutputNoteRecord,
) -> Result<(), StoreError> {
    const NOTE_QUERY: &str = "
        INSERT OR REPLACE INTO output_notes(
            note_id,
            assets,
            recipient_digest,
            metadata,
            nullifier,
            expected_height,
            state_discriminant,
            state
        ) VALUES (
            :note_id,
            :assets,
            :recipient,
            :metadata,
            :nullifier,
            :expected_height,
            :state_discriminant,
            :state
        );";

    let SerializedOutputNoteData {
        id,
        assets,
        metadata,
        nullifier,
        recipient_digest,
        expected_height,
        state_discriminant,
        state,
    } = serialize_output_note(note)?;

    tx.execute(
        NOTE_QUERY,
        named_params! {
            ":note_id": id,
            ":assets": assets,
            ":recipient": recipient_digest,
            ":metadata": metadata,
            ":nullifier": nullifier,
            ":expected_height": expected_height,
            ":state_discriminant": state_discriminant,
            ":state": state,
        },
    )?;

    Ok(())
}

/// Parse input note columns from the provided row into native types.
fn parse_input_note_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedInputNoteParts, rusqlite::Error> {
    let assets: Vec<u8> = row.get(0)?;
    let serial_number: Vec<u8> = row.get(1)?;
    let inputs: Vec<u8> = row.get(2)?;
    let script: Vec<u8> = row.get(3)?;
    let state: Vec<u8> = row.get(4)?;
    let created_at: u64 = row.get(5)?;

    Ok(SerializedInputNoteParts {
        assets,
        serial_number,
        inputs,
        script,
        state,
        created_at,
    })
}

/// Parse a note from the provided parts.
fn parse_input_note(
    serialized_input_note_parts: SerializedInputNoteParts,
) -> Result<InputNoteRecord, StoreError> {
    let SerializedInputNoteParts {
        assets,
        serial_number,
        inputs,
        script,
        state,
        created_at,
    } = serialized_input_note_parts;

    let assets = NoteAssets::read_from_bytes(&assets)?;

    let serial_number = Word::read_from_bytes(&serial_number)?;
    let script = NoteScript::read_from_bytes(&script)?;
    let inputs = NoteInputs::read_from_bytes(&inputs)?;
    let recipient = NoteRecipient::new(serial_number, script, inputs);

    let details = NoteDetails::new(assets, recipient);

    let state = InputNoteState::read_from_bytes(&state)?;

    Ok(InputNoteRecord::new(details, Some(created_at), state))
}

/// Serialize the provided input note into database compatible types.
fn serialize_input_note(note: &InputNoteRecord) -> Result<SerializedInputNoteData, StoreError> {
    let id = note.id().inner().to_string();
    let nullifier = note.nullifier().to_hex();

    let details = note.details();
    let assets = details.assets().to_bytes();
    let recipient = details.recipient();

    let serial_number = recipient.serial_num().to_bytes();
    let script = recipient.script().to_bytes();
    let inputs = recipient.inputs().to_bytes();

    let script_hash = recipient.script().hash().to_hex();

    let state_discriminant = note.state().discriminant();
    let state = note.state().to_bytes();

    Ok(SerializedInputNoteData {
        id,
        assets,
        serial_number,
        inputs,
        script_hash,
        script,
        nullifier,
        state_discriminant,
        state,
    })
}

/// Parse output note columns from the provided row into native types.
fn parse_output_note_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedOutputNoteParts, rusqlite::Error> {
    let recipient_digest: String = row.get(0)?;
    let assets: Vec<u8> = row.get(1)?;
    let metadata: Vec<u8> = row.get(2)?;
    let expected_height: u32 = row.get(3)?;
    let state: Vec<u8> = row.get(4)?;

    Ok(SerializedOutputNoteParts {
        recipient_digest,
        assets,
        metadata,
        expected_height,
        state,
    })
}

/// Parse a note from the provided parts.
fn parse_output_note(
    serialized_output_note_parts: SerializedOutputNoteParts,
) -> Result<OutputNoteRecord, StoreError> {
    let SerializedOutputNoteParts {
        recipient_digest,
        assets,
        metadata,
        expected_height,
        state,
    } = serialized_output_note_parts;

    let recipient_digest = Digest::try_from(recipient_digest)?;
    let assets = NoteAssets::read_from_bytes(&assets)?;
    let metadata = NoteMetadata::read_from_bytes(&metadata)?;
    let state = OutputNoteState::read_from_bytes(&state)?;

    Ok(OutputNoteRecord::new(
        recipient_digest,
        assets,
        metadata,
        state,
        expected_height,
    ))
}

/// Serialize the provided output note into database compatible types.
fn serialize_output_note(note: &OutputNoteRecord) -> Result<SerializedOutputNoteData, StoreError> {
    let id = note.id().inner().to_string();
    let assets = note.assets().to_bytes();
    let recipient_digest = note.recipient_digest().to_hex();
    let metadata = note.metadata().to_bytes();

    let nullifier = note.nullifier().map(|nullifier| nullifier.to_hex());

    let state_discriminant = note.state().discriminant();
    let state = note.state().to_bytes();

    Ok(SerializedOutputNoteData {
        id,
        assets,
        metadata,
        nullifier,
        recipient_digest,
        expected_height: note.expected_height(),
        state_discriminant,
        state,
    })
}

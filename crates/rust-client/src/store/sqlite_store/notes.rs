use alloc::{
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    accounts::AccountId,
    crypto::utils::{Deserializable, Serializable},
    notes::{
        NoteAssets, NoteDetails, NoteId, NoteInclusionProof, NoteInputs, NoteMetadata,
        NoteRecipient, NoteScript, Nullifier,
    },
    Digest, Word,
};
use miden_tx::utils::DeserializationError;
use rusqlite::{named_params, params, params_from_iter, types::Value, Transaction};

use super::SqliteStore;
use crate::store::{
    note_record::{
        NOTE_STATUS_COMMITTED, NOTE_STATUS_CONSUMED, NOTE_STATUS_EXPECTED, NOTE_STATUS_PROCESSING,
        STATE_COMMITTED, STATE_CONSUMED_AUTHENTICATED_LOCAL, STATE_CONSUMED_EXTERNAL,
        STATE_CONSUMED_UNAUTHENTICATED_LOCAL, STATE_EXPECTED, STATE_PROCESSING_AUTHENTICATED,
        STATE_PROCESSING_UNAUTHENTICATED, STATE_UNVERIFIED,
    },
    InputNoteRecord, NoteFilter, NoteRecordDetails, NoteState, NoteStatus, OutputNoteRecord,
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

type SerializedOutputNoteData = (
    String,
    Vec<u8>,
    String,
    String,
    Vec<u8>,
    Option<Vec<u8>>,
    Option<String>,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
    Option<u32>,
);

/// Represents the parts retrieved from the database to build an `InputNoteRecord`
struct SerializedInputNoteParts {
    pub assets: Vec<u8>,
    pub serial_number: Vec<u8>,
    pub inputs: Vec<u8>,
    pub script: Vec<u8>,
    pub state: Vec<u8>,
    pub created_at: u64,
}

type SerializedOutputNoteParts = (
    Vec<u8>,
    Option<Vec<u8>>,
    String,
    String,
    Vec<u8>,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
    Option<i64>,
    u64,
    Option<u32>,
    Option<u64>,
    Option<u32>,
);

// NOTE FILTER
// ================================================================================================

impl NoteFilter {
    /// Returns a [String] containing the query for this Filter
    fn to_query_output_notes(&self) -> String {
        let base = "SELECT
                    note.assets,
                    note.details,
                    note.recipient,
                    note.status,
                    note.metadata,
                    note.inclusion_proof,
                    script.serialized_note_script,
                    tx.account_id,
                    note.created_at,
                    note.expected_height,
                    note.submitted_at,
                    note.nullifier_height,
                    note.ignored,
                    note.imported_tag
                    from output_notes AS note
                    LEFT OUTER JOIN notes_scripts AS script
                        ON note.details IS NOT NULL AND
                        note.script_hash = script.script_hash
                    LEFT OUTER JOIN transactions AS tx
                        ON note.consumer_transaction_id IS NOT NULL AND
                        note.consumer_transaction_id = tx.id";

        match self {
            NoteFilter::All => base.to_string(),
            NoteFilter::Committed => {
                format!("{base} WHERE status = '{NOTE_STATUS_COMMITTED}' AND NOT(ignored)")
            },
            NoteFilter::Consumed => {
                format!("{base} WHERE status = '{NOTE_STATUS_CONSUMED}' AND NOT(ignored)")
            },
            NoteFilter::Expected => {
                format!("{base} WHERE status = '{NOTE_STATUS_EXPECTED}' AND NOT(ignored)")
            },
            NoteFilter::Processing => {
                format!("{base} WHERE status = '{NOTE_STATUS_PROCESSING}' AND NOT(ignored)")
            },
            NoteFilter::Ignored => format!("{base} WHERE ignored"),
            NoteFilter::Unique(_) | NoteFilter::List(_) => {
                format!("{base} WHERE note.note_id IN rarray(?)")
            },
            NoteFilter::Nullifiers(_) => {
                format!("{base} WHERE note.nullifier IN rarray(?)")
            },
            NoteFilter::Unverified => todo!("This filter shouldn't be used for output notes, once we refactor the filter system we can remove this"),
        }
    }

    fn to_query_input_notes(&self) -> String {
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

        match self {
            NoteFilter::All => base.to_string(),
            NoteFilter::Committed => {
                format!("{base} WHERE state_discriminant = {STATE_COMMITTED}")
            },
            NoteFilter::Consumed => {
                format!(
                    "{base} WHERE state_discriminant in ({}, {}, {})",
                    STATE_CONSUMED_AUTHENTICATED_LOCAL,
                    STATE_CONSUMED_UNAUTHENTICATED_LOCAL,
                    STATE_CONSUMED_EXTERNAL
                )
            },
            NoteFilter::Expected => {
                format!("{base} WHERE state_discriminant = {}", STATE_EXPECTED)
            },
            NoteFilter::Processing => {
                format!(
                    "{base} WHERE state_discriminant in ({}, {})",
                    STATE_PROCESSING_AUTHENTICATED, STATE_PROCESSING_UNAUTHENTICATED
                )
            },
            NoteFilter::Unverified => {
                format!("{base} WHERE state_discriminant = {}", STATE_UNVERIFIED)
            },
            NoteFilter::Unique(_) | NoteFilter::List(_) => {
                format!("{base} WHERE note.note_id IN rarray(?)")
            },
            NoteFilter::Nullifiers(_) => {
                format!("{base} WHERE note.nullifier IN rarray(?)")
            },
            NoteFilter::Ignored =>  todo!("This filter shouldn't be used for input notes, once we refactor the filter system we can remove this"),
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
        match &filter {
            NoteFilter::Unique(note_id) => {
                let note_ids_list = vec![Value::Text(note_id.inner().to_string())];
                params.push(Rc::new(note_ids_list));
            },
            NoteFilter::List(ref note_ids) => {
                let note_ids_list = note_ids
                    .iter()
                    .map(|note_id| Value::Text(note_id.inner().to_string()))
                    .collect::<Vec<Value>>();

                params.push(Rc::new(note_ids_list))
            },
            NoteFilter::Nullifiers(nullifiers) => {
                let nullifiers_list = nullifiers
                    .iter()
                    .map(|nullifier| Value::Text(nullifier.to_string()))
                    .collect::<Vec<Value>>();

                params.push(Rc::new(nullifiers_list))
            },
            _ => {},
        }
        let notes = self
            .db()
            .prepare(&filter.to_query_input_notes())?
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
            NoteFilter::List(ref note_ids) => {
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
            .prepare(&filter.to_query_output_notes())?
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

    pub(crate) fn upsert_input_note(&self, note: InputNoteRecord) -> Result<(), StoreError> {
        let mut db = self.db();
        let tx = db.transaction()?;

        upsert_input_note_tx(&tx, &note)?;

        Ok(tx.commit()?)
    }

    pub(crate) fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        const QUERY: &str =
            "SELECT nullifier FROM input_notes WHERE state_discriminant NOT IN rarray(?)";
        let unspent_filters = Rc::new(vec![
            Value::from(STATE_CONSUMED_AUTHENTICATED_LOCAL.to_string()),
            Value::from(STATE_CONSUMED_UNAUTHENTICATED_LOCAL.to_string()),
            Value::from(STATE_CONSUMED_EXTERNAL.to_string()),
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
pub fn insert_output_note_tx(
    tx: &Transaction<'_>,
    block_num: u32,
    note: &OutputNoteRecord,
) -> Result<(), StoreError> {
    const NOTE_QUERY: &str = "
        INSERT INTO output_notes(
            note_id,
            assets,
            recipient,
            status,
            metadata,
            nullifier,
            script_hash,
            details,
            inclusion_proof,
            consumer_transaction_id,
            created_at,
            expected_height,
            ignored,
            imported_tag,
            nullifier_height
        ) VALUES (
            :note_id,
            :assets,
            :recipient,
            :status,
            :metadata,
            :nullifier,
            :script_hash,
            :details,
            :inclusion_proof,
            :consumer_transaction_id,
            unixepoch(current_timestamp),
            :expected_height,
            :ignored,
            :imported_tag,
            :nullifier_height);
    ";
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
        expected_height,
    ) = serialize_output_note(note)?;

    let nullifier: Option<String> = match details {
        Some(ref bytes) => NoteRecordDetails::read_from_bytes(bytes)
            .map(|details| details.nullifier().to_string())
            .ok(),
        None => None,
    };

    tx.execute(
        NOTE_QUERY,
        named_params! {
            ":note_id": note_id,
            ":assets": assets,
            ":recipient": recipient,
            ":status": status,
            ":metadata": metadata,
            ":nullifier": nullifier,
            ":script_hash": note_script_hash,
            ":details": details,
            ":inclusion_proof": inclusion_proof,
            ":expected_height": expected_height.unwrap_or(block_num),
            ":ignored": false,
            ":imported_tag": None::<u32>,
        },
    )?;

    if note_script_hash.is_some() {
        const QUERY: &str =
            "INSERT OR REPLACE INTO notes_scripts (script_hash, serialized_note_script) VALUES (?, ?)";
        tx.execute(QUERY, params![note_script_hash, serialized_note_script,])?;
    }

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

    let state = NoteState::read_from_bytes(&state)?;

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

/// Parse input note columns from the provided row into native types.
fn parse_output_note_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedOutputNoteParts, rusqlite::Error> {
    let assets: Vec<u8> = row.get(0)?;
    let details: Option<Vec<u8>> = row.get(1)?;
    let recipient: String = row.get(2)?;
    let status: String = row.get(3)?;
    let metadata: Vec<u8> = row.get(4)?;
    let inclusion_proof: Option<Vec<u8>> = row.get(5)?;
    let serialized_note_script: Option<Vec<u8>> = row.get(6)?;
    let consumer_account_id: Option<i64> = row.get(7)?;
    let created_at: u64 = row.get(8)?;
    let expected_height: Option<u32> = row.get(9)?;
    let submitted_at: Option<u64> = row.get(10)?;
    let nullifier_height: Option<u32> = row.get(11)?;

    Ok((
        assets,
        details,
        recipient,
        status,
        metadata,
        inclusion_proof,
        serialized_note_script,
        consumer_account_id,
        created_at,
        expected_height,
        submitted_at,
        nullifier_height,
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
        _serialized_note_script,
        consumer_account_id,
        created_at,
        expected_height,
        submitted_at,
        nullifier_height,
    ) = serialized_output_note_parts;

    let note_details: Option<NoteRecordDetails> = if let Some(details_as_bytes) = note_details {
        let note_details = NoteRecordDetails::read_from_bytes(&details_as_bytes)
            .map_err(StoreError::DataDeserializationError)?;

        Some(note_details)
    } else {
        None
    };

    let note_metadata = NoteMetadata::read_from_bytes(&note_metadata)
        .map_err(StoreError::DataDeserializationError)?;

    let note_assets = NoteAssets::read_from_bytes(&note_assets)?;

    let inclusion_proof = match note_inclusion_proof {
        Some(note_inclusion_proof) => {
            let note_inclusion_proof = NoteInclusionProof::read_from_bytes(&note_inclusion_proof)
                .map_err(StoreError::DataDeserializationError)?;

            Some(note_inclusion_proof)
        },
        _ => None,
    };

    let recipient = Digest::try_from(recipient)?;
    let id = NoteId::new(recipient, note_assets.commitment());

    let consumer_account_id: Option<AccountId> = match consumer_account_id {
        Some(account_id) => Some(AccountId::try_from(account_id as u64)?),
        None => None,
    };

    // If the note is committed and has a consumer account id, then it was consumed locally but the
    // client is not synced with the chain
    let status = match status.as_str() {
        NOTE_STATUS_EXPECTED => NoteStatus::Expected {
            created_at: Some(created_at),
            block_height: expected_height,
        },
        NOTE_STATUS_COMMITTED => NoteStatus::Committed {
            block_height: inclusion_proof
                .clone()
                .map(|proof| proof.location().block_num())
                .expect("Committed note should have inclusion proof"),
        },
        NOTE_STATUS_PROCESSING => NoteStatus::Processing {
            consumer_account_id: consumer_account_id
                .expect("Processing note should have consumer account id"),
            submitted_at: submitted_at.expect("Processing note should have submition timestamp"),
        },
        NOTE_STATUS_CONSUMED => NoteStatus::Consumed {
            consumer_account_id,
            block_height: nullifier_height.expect("Consumed note should have nullifier height"),
        },
        _ => {
            return Err(StoreError::DataDeserializationError(DeserializationError::InvalidValue(
                format!("NoteStatus: {}", status),
            )))
        },
    };

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
            let inclusion_proof = proof.to_bytes();
            let status = NOTE_STATUS_COMMITTED.to_string();

            (Some(inclusion_proof), status)
        },
        None => {
            let status = NOTE_STATUS_EXPECTED.to_string();

            (None, status)
        },
    };
    let recipient = note.recipient().to_hex();

    let metadata = note.metadata().to_bytes();

    let details = note.details().map(|details| details.to_bytes());

    let note_script_hash = note.details().map(|details| details.script_hash().to_hex());
    let serialized_note_script = note.details().map(|details| details.script().to_bytes());
    let expected_height = match note.status() {
        NoteStatus::Expected { block_height, .. } => block_height,
        _ => None,
    };

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
        expected_height,
    ))
}

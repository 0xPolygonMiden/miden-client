use std::fmt;

use crate::errors::StoreError;
use crate::store::{InputNoteRecord, NoteFilter};

use super::SqliteStore;

use clap::error::Result;

use crypto::utils::{Deserializable, Serializable};

use objects::notes::{Note, NoteAssets, NoteId, NoteInclusionProof, NoteInputs, NoteScript};

use objects::{accounts::AccountId, notes::NoteMetadata, Felt};
use rusqlite::{params, Transaction};

const P2ID_NOTE_SCRIPT_ROOT: &str =
    "0x65c08aef0e3d11ce8a26662005a5272398e8810e5e13a903a993ee622d03675f";
const P2IDR_NOTE_SCRIPT_ROOT: &str =
    "0x03dd8f8fd57f015d821648292cee0ce42e16c4b80427c46b9cb874db44395f47";

fn insert_note_query(table_name: NoteTable) -> String {
    format!("\
    INSERT INTO {table_name}
        (note_id, nullifier, script, assets, inputs, serial_num, sender_id, tag, inclusion_proof, recipient, status, script_hash)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
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
    fn to_query(&self, notes_table: NoteTable) -> String {
        let base = format!("SELECT script, inputs, assets, serial_num, sender_id, tag, inclusion_proof FROM {notes_table}");
        match self {
            NoteFilter::All => base,
            NoteFilter::Committed => format!("{base} WHERE status = 'committed'"),
            NoteFilter::Consumed => format!("{base} WHERE status = 'consumed'"),
            NoteFilter::Pending => format!("{base} WHERE status = 'pending'"),
            NoteFilter::ConsumableBy(_) => {
                format!("{base} WHERE status = 'committed' AND (script_hash = '{P2ID_NOTE_SCRIPT_ROOT}' OR script_hash = '{P2IDR_NOTE_SCRIPT_ROOT}')")
            }
        }
    }

    /// Returns a list of parameters for each type of filter
    fn query_params(&self) -> Vec<rusqlite::types::Value> {
        match self {
            NoteFilter::ConsumableBy(account_id) => {
                let inputs = NoteInputs::new(vec![(*account_id).into()])
                    .expect("Only one argument should not cause errors");
                let _inputs_param = inputs.to_bytes();
                // vec![rusqlite::types::Value::Blob(inputs_param)]
                vec![]
            }
            _ => vec![],
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
            .query_map(
                rusqlite::params_from_iter(filter.query_params()),
                parse_input_note_columns,
            )
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).and_then(parse_input_note))
            .collect::<Result<Vec<InputNoteRecord>, _>>()
    }

    /// Retrieves the output notes from the database
    pub(crate) fn get_output_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.db
            .prepare(&filter.to_query(NoteTable::OutputNotes))?
            .query_map(
                rusqlite::params_from_iter(filter.query_params()),
                parse_input_note_columns,
            )
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).and_then(parse_input_note))
            .collect::<Result<Vec<InputNoteRecord>, _>>()
    }

    pub(crate) fn get_input_note(&self, note_id: NoteId) -> Result<InputNoteRecord, StoreError> {
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
        script_hash,
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
            script_hash
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
        script_hash,
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
            script_hash
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
    let script_hash = note.note().script().hash();
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
        script_hash.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use crypto::{rand::RpoRandomCoin, Felt};
    use miden_lib::notes::{create_p2id_note, create_p2idr_note};
    use mock::constants::{generate_account_seed, AccountSeedType};
    use objects::{accounts::AccountId, assets::FungibleAsset};
    use rand::Rng;

    use crate::store::sqlite_store::notes::{P2IDR_NOTE_SCRIPT_ROOT, P2ID_NOTE_SCRIPT_ROOT};

    // We need to make sure the script roots we use for filters are in line with the note scripts
    // coming from Miden objects
    #[test]
    fn ensure_correct_script_roots() {
        // create dummy data for the notes
        let faucet_id: AccountId = 10347894387879516201u64.try_into().unwrap();
        let (account_id, _) =
            generate_account_seed(AccountSeedType::RegularAccountUpdatableCodeOnChain);

        let rng = {
            let mut rng = rand::thread_rng();
            let coin_seed: [u64; 4] = rng.gen();
            RpoRandomCoin::new(coin_seed.map(Felt::new))
        };

        // create dummy notes to compare note script roots
        let p2id_note = create_p2id_note(
            account_id,
            account_id,
            vec![FungibleAsset::new(faucet_id, 100u64).unwrap().into()],
            rng,
        )
        .unwrap();
        let p2idr_note = create_p2idr_note(
            account_id,
            account_id,
            vec![FungibleAsset::new(faucet_id, 100u64).unwrap().into()],
            10,
            rng,
        )
        .unwrap();

        assert_eq!(p2id_note.script().hash().to_string(), P2ID_NOTE_SCRIPT_ROOT);
        assert_eq!(
            p2idr_note.script().hash().to_string(),
            P2IDR_NOTE_SCRIPT_ROOT
        );
    }
}

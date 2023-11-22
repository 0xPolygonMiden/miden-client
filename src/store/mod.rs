use super::{errors::StoreError, AccountStub, ClientConfig};
use crypto::{utils::collections::BTreeMap, Word};
use objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountVault},
    assembly::AstSerdeOptions,
    assets::Asset,
    notes::{Note, NoteMetadata, RecordedNote},
    Digest, Felt,
};
use rusqlite::{params, Connection};

mod migrations;

// TYPES
// ================================================================================================

type SerializedInputNoteData = (
    String,
    String,
    String,
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

type SerializedInputNoteParts = (String, String, String, String, u64, u64, u64, String);

// CLIENT STORE
// ================================================================================================

pub struct Store {
    db: Connection,
}

impl Store {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Store] instantiated with the specified configuration options.
    pub fn new(config: StoreConfig) -> Result<Self, StoreError> {
        let mut db = Connection::open(config.path).map_err(StoreError::ConnectionError)?;
        migrations::update_to_latest(&mut db)?;

        Ok(Self { db })
    }

    // ACCOUNTS
    // --------------------------------------------------------------------------------------------

    pub fn get_accounts(&self) -> Result<Vec<AccountStub>, StoreError> {
        let mut stmt = self
            .db
            .prepare("SELECT id, nonce, vault_root, storage_root, code_root FROM accounts")
            .map_err(StoreError::QueryError)?;

        let mut rows = stmt.query([]).map_err(StoreError::QueryError)?;
        let mut result = Vec::new();
        while let Some(row) = rows.next().map_err(StoreError::QueryError)? {
            // TODO: implement proper error handling and conversions

            let id: i64 = row.get(0).map_err(StoreError::QueryError)?;
            let nonce: i64 = row.get(1).map_err(StoreError::QueryError)?;

            let vault_root: String = row.get(2).map_err(StoreError::QueryError)?;
            let storage_root: String = row.get(3).map_err(StoreError::QueryError)?;
            let code_root: String = row.get(4).map_err(StoreError::QueryError)?;

            result.push(AccountStub::new(
                (id as u64)
                    .try_into()
                    .expect("Conversion from stored AccountID should not panic"),
                (nonce as u64).into(),
                serde_json::from_str(&vault_root).map_err(StoreError::DataDeserializationError)?,
                serde_json::from_str(&storage_root)
                    .map_err(StoreError::DataDeserializationError)?,
                serde_json::from_str(&code_root).map_err(StoreError::DataDeserializationError)?,
            ));
        }

        Ok(result)
    }

    pub fn insert_account(&self, account: &Account) -> Result<(), StoreError> {
        let id: u64 = account.id().into();
        let code_root = serde_json::to_string(&account.code().root())
            .map_err(StoreError::InputSerializationError)?;
        let storage_root = serde_json::to_string(&account.storage().root())
            .map_err(StoreError::InputSerializationError)?;
        let vault_root = serde_json::to_string(&account.vault().commitment())
            .map_err(StoreError::InputSerializationError)?;

        self.db.execute(
            "INSERT INTO accounts (id, code_root, storage_root, vault_root, nonce, committed) VALUES (?, ?, ?, ?, ?, ?)",
            params![
                id as i64,
                code_root,
                storage_root,
                vault_root,
                account.nonce().inner() as i64,
                account.is_on_chain(),
            ],
        )
        .map(|_| ())
        .map_err(StoreError::QueryError)
    }

    pub fn insert_account_code(&self, account_code: &AccountCode) -> Result<(), StoreError> {
        let code_root = serde_json::to_string(&account_code.root())
            .map_err(StoreError::InputSerializationError)?;
        let code = serde_json::to_string(account_code.procedures())
            .map_err(StoreError::InputSerializationError)?;
        let module = account_code.module().to_bytes(AstSerdeOptions {
            serialize_imports: true,
        });

        self.db
            .execute(
                "INSERT INTO account_code (root, procedures, module) VALUES (?, ?, ?)",
                params![code_root, code, module,],
            )
            .map(|_| ())
            .map_err(StoreError::QueryError)
    }

    pub fn insert_account_storage(
        &self,
        account_storage: &AccountStorage,
    ) -> Result<(), StoreError> {
        let storage_root = serde_json::to_string(&account_storage.root())
            .map_err(StoreError::InputSerializationError)?;

        let storage_slots: BTreeMap<u64, &Word> = account_storage.slots().leaves().collect();
        let storage_slots =
            serde_json::to_string(&storage_slots).map_err(StoreError::InputSerializationError)?;

        self.db
            .execute(
                "INSERT INTO account_storage (root, slots) VALUES (?, ?)",
                params![storage_root, storage_slots],
            )
            .map(|_| ())
            .map_err(StoreError::QueryError)
    }

    pub fn insert_account_vault(&self, account_vault: &AccountVault) -> Result<(), StoreError> {
        let vault_root = serde_json::to_string(&account_vault.commitment())
            .map_err(StoreError::InputSerializationError)?;

        let assets: Vec<Asset> = account_vault.assets().collect();
        let assets = serde_json::to_string(&assets).map_err(StoreError::InputSerializationError)?;

        self.db
            .execute(
                "INSERT INTO account_vaults (root, assets) VALUES (?, ?)",
                params![vault_root, assets],
            )
            .map(|_| ())
            .map_err(StoreError::QueryError)
    }

    // NOTES
    // --------------------------------------------------------------------------------------------

    /// Retrieves the input notes from the database
    pub fn get_input_notes(&self) -> Result<Vec<RecordedNote>, StoreError> {
        const QUERY: &str = "SELECT script, inputs, vault, serial_num, sender_id, tag, num_assets, inclusion_proof FROM input_notes";

        self.db
            .prepare(QUERY)
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
}

// STORE CONFIG
// ================================================================================================

pub struct StoreConfig {
    path: String,
}

impl From<&ClientConfig> for StoreConfig {
    fn from(config: &ClientConfig) -> Self {
        Self {
            path: config.store_path.clone(),
        }
    }
}

// HELPERS
// ================================================================================================
/// Parse input note columns from the provided row into native types.
fn parse_input_note_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedInputNoteParts, rusqlite::Error> {
    let script: String = row.get(0)?;
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
    let script = serde_json::from_str(&script).map_err(StoreError::DataDeserializationError)?;
    let inputs = serde_json::from_str(&inputs).map_err(StoreError::DataDeserializationError)?;
    let vault = serde_json::from_str(&vault).map_err(StoreError::DataDeserializationError)?;
    let serial_num =
        serde_json::from_str(&serial_num).map_err(StoreError::DataDeserializationError)?;
    let note_metadata = NoteMetadata::new(
        AccountId::new_unchecked(Felt::new(sender_id)),
        Felt::new(tag),
        Felt::new(num_assets),
    );
    let note = Note::from_parts(script, inputs, vault, serial_num, note_metadata);

    let inclusion_proof =
        serde_json::from_str(&inclusion_proof).map_err(StoreError::DataDeserializationError)?;
    Ok(RecordedNote::new(note, inclusion_proof))
}

/// Serialize the provided input note into database compatible types.
fn serialize_input_note(
    recorded_note: &RecordedNote,
) -> Result<SerializedInputNoteData, StoreError> {
    let hash = serde_json::to_string(&recorded_note.note().hash())
        .map_err(StoreError::InputSerializationError)?;
    let nullifier = serde_json::to_string(&recorded_note.note().nullifier())
        .map_err(StoreError::InputSerializationError)?;
    let script = serde_json::to_string(&recorded_note.note().script())
        .map_err(StoreError::InputSerializationError)?;
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
    let commit_height = recorded_note.origin().block_num.inner() as i64;
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

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use std::env::temp_dir;
    use uuid::Uuid;

    pub fn create_test_store_path() -> std::path::PathBuf {
        let mut temp_file = temp_dir();
        temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
        temp_file
    }
}

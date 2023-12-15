use super::errors::StoreError;
use crate::config::ClientConfig;

use clap::error::Result;
use crypto::hash::rpo::RpoDigest;
use crypto::{
    dsa::rpo_falcon512::KeyPair,
    utils::{collections::BTreeMap, Deserializable, Serializable},
    Word,
};

use objects::accounts::AccountStub;
use objects::notes::NoteScript;
use objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountVault},
    assembly::{AstSerdeOptions, ModuleAst},
    assets::Asset,
    notes::{Note, NoteMetadata, RecordedNote},
    Digest, Felt,
};
use rusqlite::{params, Connection, Transaction};

pub(crate) mod mock_executor_data_store;
pub mod transactions;

mod migrations;

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

type SerializedAccountData = (i64, String, String, String, i64, bool);
type SerializedAccountsParts = (i64, i64, String, String, String);

type SerializedAccountAuthData = (i64, Vec<u8>);
type SerializedAccountAuthParts = (i64, String);

type SerializedAccountVaultData = (String, String);
type SerializedAccountVaultParts = (String, String);

type SerializedAccountCodeData = (String, String, Vec<u8>);
type SerializedAccountCodeParts = (String, String, String);

type SerializedAccountStorageData = (String, String);
type SerializedAccountStorageParts = (String, String);

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

    /// Returns the account id's of all accounts stored in the database
    pub fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        const QUERY: &str = "SELECT id FROM accounts";

        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .map(|id: i64| AccountId::try_from(id as u64).expect("account id is valid"))
            })
            .collect::<Result<Vec<AccountId>, _>>()
    }

    pub fn get_accounts(&self) -> Result<Vec<AccountStub>, StoreError> {
        const QUERY: &str = "SELECT id, nonce, vault_root, storage_root, code_root FROM accounts";
        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map([], parse_accounts_columns)
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_accounts)
            })
            .collect()
    }

    pub fn get_account_by_id(&self, account_id: AccountId) -> Result<AccountStub, StoreError> {
        let account_id_int: u64 = account_id.into();
        const QUERY: &str =
            "SELECT id, nonce, vault_root, storage_root, code_root FROM accounts WHERE id = ?";
        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map(params![account_id_int as i64], parse_accounts_columns)
            .map_err(StoreError::QueryError)?
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_accounts)
            })
            .next()
            .ok_or(StoreError::AccountDataNotFound(account_id))?
    }

    /// Retrieve account keys data by Account Id
    pub fn get_account_auth(&self, account_id: AccountId) -> Result<AuthInfo, StoreError> {
        let account_id_int: u64 = account_id.into();
        const QUERY: &str = "SELECT account_id, auth_info FROM account_auth WHERE account_id = ?";
        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map(params![account_id_int as i64], parse_account_auth_columns)
            .map_err(StoreError::QueryError)?
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_account_auth)
            })
            .next()
            .ok_or(StoreError::AccountDataNotFound(account_id))?
    }

    /// Retrieve account code-related data by code root
    pub fn get_account_code(
        &self,
        root: Digest,
    ) -> Result<(Vec<RpoDigest>, ModuleAst), StoreError> {
        let root_serialized =
            serde_json::to_string(&root).map_err(StoreError::InputSerializationError)?;
        const QUERY: &str = "SELECT procedures, module FROM account_code WHERE root = ?";

        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map(params![root_serialized], parse_account_code_columns)
            .map_err(StoreError::QueryError)?
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_account_code)
            })
            .next()
            .ok_or(StoreError::AccountCodeDataNotFound(root))?
    }

    /// Retrieve account storage data by vault root
    pub fn get_account_storage(&self, root: RpoDigest) -> Result<BTreeMap<u64, Word>, StoreError> {
        let root_serialized =
            serde_json::to_string(&root).map_err(StoreError::InputSerializationError)?;

        const QUERY: &str = "SELECT root, slots FROM account_storage WHERE root = ?";
        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map(params![root_serialized], parse_account_storage_columns)
            .map_err(StoreError::QueryError)?
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_account_storage)
            })
            .next()
            .ok_or(StoreError::AccountStorageNotFound(root))?
    }

    /// Retrieve assets by vault root
    pub fn get_vault_assets(&self, root: RpoDigest) -> Result<Vec<Asset>, StoreError> {
        let vault_root =
            serde_json::to_string(&root).map_err(StoreError::InputSerializationError)?;

        const QUERY: &str = "SELECT assets FROM account_vaults WHERE root = ?";
        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map(params![vault_root], parse_account_vault_columns)
            .map_err(StoreError::QueryError)?
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_account_vault)
            })
            .next()
            .ok_or(StoreError::VaultDataNotFound(root))?
    }

    pub fn insert_account(
        &mut self,
        account: &Account,
        auth_info: &AuthInfo,
    ) -> Result<(), StoreError> {
        let tx = self
            .db
            .transaction()
            .map_err(StoreError::TransactionError)?;

        Self::insert_account_code(&tx, account.code())?;
        Self::insert_account_storage(&tx, account.storage())?;
        Self::insert_account_vault(&tx, account.vault())?;
        Self::insert_account_record(&tx, account)?;
        Self::insert_account_auth(&tx, account.id(), auth_info)?;

        tx.commit().map_err(StoreError::TransactionError)
    }

    fn insert_account_record(tx: &Transaction<'_>, account: &Account) -> Result<(), StoreError> {
        let (id, code_root, storage_root, vault_root, nonce, committed) =
            serialize_account(account)?;
        const QUERY: &str =  "INSERT INTO accounts (id, code_root, storage_root, vault_root, nonce, committed) VALUES (?, ?, ?, ?, ?, ?)";
        tx.execute(
            QUERY,
            params![id, code_root, storage_root, vault_root, nonce, committed],
        )
        .map(|_| ())
        .map_err(StoreError::QueryError)
    }

    fn insert_account_code(
        tx: &Transaction<'_>,
        account_code: &AccountCode,
    ) -> Result<(), StoreError> {
        let (code_root, code, module) = serialize_account_code(account_code)?;

        const QUERY: &str =
            "INSERT OR IGNORE INTO account_code (root, procedures, module) VALUES (?, ?, ?)";
        tx.execute(QUERY, params![code_root, code, module,])
            .map(|_| ())
            .map_err(StoreError::QueryError)
    }

    fn insert_account_storage(
        tx: &Transaction<'_>,
        account_storage: &AccountStorage,
    ) -> Result<(), StoreError> {
        let (storage_root, storage_slots) = serialize_account_storage(account_storage)?;
        const QUERY: &str = "INSERT INTO account_storage (root, slots) VALUES (?, ?)";
        tx.execute(QUERY, params![storage_root, storage_slots])
            .map(|_| ())
            .map_err(StoreError::QueryError)
    }

    fn insert_account_vault(
        tx: &Transaction<'_>,
        account_vault: &AccountVault,
    ) -> Result<(), StoreError> {
        let (vault_root, assets) = serialize_account_vault(account_vault)?;
        const QUERY: &str = "INSERT OR IGNORE INTO account_vaults (root, assets) VALUES (?, ?)";
        tx.execute(QUERY, params![vault_root, assets])
            .map(|_| ())
            .map_err(StoreError::QueryError)
    }

    pub fn insert_account_auth(
        tx: &Transaction<'_>,
        account_id: AccountId,
        auth_info: &AuthInfo,
    ) -> Result<(), StoreError> {
        let (account_id, auth_info) = serialize_account_auth(account_id, auth_info)?;
        const QUERY: &str = "INSERT INTO account_auth (account_id, auth_info) VALUES (?, ?)";
        tx.execute(QUERY, params![account_id, auth_info])
            .map(|_| ())
            .map_err(StoreError::QueryError)
    }

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
                    .and_then(|v: String| {
                        serde_json::from_str(&v).map_err(StoreError::JsonDataDeserializationError)
                    })
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
    pub fn add_note_tag(&mut self, tag: u64) -> Result<(), StoreError> {
        let mut tags = self.get_note_tags()?;
        tags.push(tag);
        let tags = serde_json::to_string(&tags).map_err(StoreError::InputSerializationError)?;

        const QUERY: &str = "UPDATE state_sync SET tags = ?";
        self.db
            .execute(QUERY, params![tags])
            .map_err(StoreError::QueryError)
            .map(|_| ())
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
            let nullifier =
                serde_json::to_string(&nullifier).map_err(StoreError::InputSerializationError)?;
            tx.execute(SPENT_QUERY, params![nullifier])
                .map_err(StoreError::QueryError)?;
        }

        // commit the transaction
        tx.commit().map_err(StoreError::QueryError)
    }
}

// DATABASE AUTH INFO
// ================================================================================================

/// Type of Authentication Methods supported by the DB
///
/// TODO: add remaining auth types
pub enum AuthInfo {
    RpoFalcon512(KeyPair),
}

const RPO_FALCON512_AUTH: u8 = 0;

impl AuthInfo {
    /// Returns byte identifier of specific AuthInfo
    fn type_byte(&self) -> u8 {
        match self {
            AuthInfo::RpoFalcon512(_) => RPO_FALCON512_AUTH,
        }
    }
}

impl Serializable for AuthInfo {
    fn write_into<W: crypto::utils::ByteWriter>(&self, target: &mut W) {
        let mut bytes = vec![self.type_byte()];
        match self {
            AuthInfo::RpoFalcon512(key_pair) => {
                bytes.append(&mut key_pair.to_bytes());
                target.write_bytes(&bytes);
            }
        }
    }
}

impl Deserializable for AuthInfo {
    fn read_from<R: crypto::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, crypto::utils::DeserializationError> {
        let auth_type: u8 = source.read_u8()?;
        match auth_type {
            RPO_FALCON512_AUTH => {
                let key_pair = KeyPair::read_from(source)?;
                Ok(AuthInfo::RpoFalcon512(key_pair))
            }
            val => Err(crypto::utils::DeserializationError::InvalidValue(
                val.to_string(),
            )),
        }
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

// HELPERS
// ================================================================================================

/// Parse accounts colums from the provided row into native types
fn parse_accounts_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedAccountsParts, rusqlite::Error> {
    let id: i64 = row.get(0)?;
    let nonce: i64 = row.get(1)?;
    let vault_root: String = row.get(2)?;
    let storage_root: String = row.get(3)?;
    let code_root: String = row.get(4)?;
    Ok((id, nonce, vault_root, storage_root, code_root))
}

/// Parse an account from the provided parts.
fn parse_accounts(
    serialized_account_parts: SerializedAccountsParts,
) -> Result<AccountStub, StoreError> {
    let (id, nonce, vault_root, storage_root, code_root) = serialized_account_parts;

    Ok(AccountStub::new(
        (id as u64)
            .try_into()
            .expect("Conversion from stored AccountID should not panic"),
        (nonce as u64).into(),
        serde_json::from_str(&vault_root).map_err(StoreError::JsonDataDeserializationError)?,
        serde_json::from_str(&storage_root).map_err(StoreError::JsonDataDeserializationError)?,
        serde_json::from_str(&code_root).map_err(StoreError::JsonDataDeserializationError)?,
    ))
}

/// Serialized the provided account into database compatible types.
fn serialize_account(account: &Account) -> Result<SerializedAccountData, StoreError> {
    let id: u64 = account.id().into();
    let code_root = serde_json::to_string(&account.code().root())
        .map_err(StoreError::InputSerializationError)?;
    let storage_root = serde_json::to_string(&account.storage().root())
        .map_err(StoreError::InputSerializationError)?;
    let vault_root = serde_json::to_string(&account.vault().commitment())
        .map_err(StoreError::InputSerializationError)?;
    let committed = account.is_on_chain();
    let nonce = account.nonce().inner() as i64;

    Ok((
        id as i64,
        code_root,
        storage_root,
        vault_root,
        nonce,
        committed,
    ))
}

/// Parse account_auth columns from the provided row into native types
fn parse_account_auth_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedAccountAuthParts, rusqlite::Error> {
    let account_id: i64 = row.get(0)?;
    let auth_info_bytes: String = row.get(1)?;
    Ok((account_id, auth_info_bytes))
}

/// Parse an `AuthInfo` from the provided parts.
fn parse_account_auth(
    serialized_account_auth_parts: SerializedAccountAuthParts,
) -> Result<AuthInfo, StoreError> {
    let (_, auth_info_bytes) = serialized_account_auth_parts;
    let auth_info = AuthInfo::read_from_bytes(auth_info_bytes.as_bytes())
        .map_err(StoreError::DataDeserializationError)?;
    Ok(auth_info)
}

/// Serialized the provided account_auth into database compatible types.
fn serialize_account_auth(
    account_id: AccountId,
    auth_info: &AuthInfo,
) -> Result<SerializedAccountAuthData, StoreError> {
    let account_id: u64 = account_id.into();
    let auth_info = auth_info.to_bytes();
    Ok((account_id as i64, auth_info))
}

/// Parse account_code columns from the provided row into native types.
fn parse_account_code_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedAccountCodeParts, rusqlite::Error> {
    let root: String = row.get(0)?;
    let procedures: String = row.get(1)?;
    let module: String = row.get(2)?;
    Ok((root, procedures, module))
}

/// Parse an account_code from the provided parts.
fn parse_account_code(
    serialized_account_code_parts: SerializedAccountCodeParts,
) -> Result<(Vec<RpoDigest>, ModuleAst), StoreError> {
    let (_, procedures, module) = serialized_account_code_parts;

    let procedures =
        serde_json::from_str(&procedures).map_err(StoreError::JsonDataDeserializationError)?;
    let module =
        ModuleAst::from_bytes(module.as_bytes()).map_err(StoreError::DataDeserializationError)?;
    Ok((procedures, module))
}

/// Serialize the provided account_code into database compatible types.
fn serialize_account_code(
    account_code: &AccountCode,
) -> Result<SerializedAccountCodeData, StoreError> {
    let root =
        serde_json::to_string(&account_code.root()).map_err(StoreError::InputSerializationError)?;
    let procedures = serde_json::to_string(account_code.procedures())
        .map_err(StoreError::InputSerializationError)?;
    let module = account_code.module().to_bytes(AstSerdeOptions {
        serialize_imports: true,
    });

    Ok((root, procedures, module))
}

/// Parse account_storage columns from the provided row into native types.
fn parse_account_storage_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedAccountStorageParts, rusqlite::Error> {
    let root: String = row.get(0)?;
    let slots: String = row.get(1)?;
    Ok((root, slots))
}

/// Parse an account_storage from the provided parts.
fn parse_account_storage(
    serialized_account_storage_parts: SerializedAccountStorageParts,
) -> Result<BTreeMap<u64, Word>, StoreError> {
    let (_, slots) = serialized_account_storage_parts;

    let slots = serde_json::from_str(&slots).map_err(StoreError::JsonDataDeserializationError)?;
    Ok(slots)
}

/// Serialize the provided account_storage into database compatible types.
fn serialize_account_storage(
    account_storage: &AccountStorage,
) -> Result<SerializedAccountStorageData, StoreError> {
    let root = serde_json::to_string(&account_storage.root())
        .map_err(StoreError::InputSerializationError)?;
    let slots: BTreeMap<u64, &Word> = account_storage.slots().leaves().collect();
    let slots = serde_json::to_string(&slots).map_err(StoreError::InputSerializationError)?;
    Ok((root, slots))
}

/// Parse account_vault columns from the provided row into native types.
fn parse_account_vault_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedAccountVaultParts, rusqlite::Error> {
    let root: String = row.get(0)?;
    let assets: String = row.get(1)?;
    Ok((root, assets))
}

/// Parse a vector of assets from the provided parts.
fn parse_account_vault(
    serialized_account_vault_parts: SerializedAccountStorageParts,
) -> Result<Vec<Asset>, StoreError> {
    let (_, assets) = serialized_account_vault_parts;

    let assets = serde_json::from_str(&assets).map_err(StoreError::JsonDataDeserializationError)?;
    Ok(assets)
}

/// Serialize the provided account_vault into database compatible types.
fn serialize_account_vault(
    account_vault: &AccountVault,
) -> Result<SerializedAccountVaultData, StoreError> {
    let root = serde_json::to_string(&account_vault.commitment())
        .map_err(StoreError::InputSerializationError)?;
    let assets: Vec<Asset> = account_vault.assets().collect();
    let assets = serde_json::to_string(&assets).map_err(StoreError::InputSerializationError)?;
    Ok((root, assets))
}

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
    let nullifier = serde_json::to_string(&recorded_note.note().nullifier())
        .map_err(StoreError::InputSerializationError)?;
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
    use crypto::{
        dsa::rpo_falcon512::KeyPair,
        utils::{Deserializable, Serializable},
    };
    use std::env::temp_dir;
    use uuid::Uuid;

    use super::AuthInfo;
    use miden_lib::assembler::assembler;
    use mock::mock::account;
    use rusqlite::Connection;

    use crate::store;

    use super::{migrations, Store};

    pub fn create_test_store_path() -> std::path::PathBuf {
        let mut temp_file = temp_dir();
        temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
        temp_file
    }

    fn create_test_store() -> Store {
        let temp_file = create_test_store_path();
        let mut db = Connection::open(temp_file).unwrap();
        migrations::update_to_latest(&mut db).unwrap();

        Store { db }
    }
    #[test]
    fn test_auth_info_serialization() {
        let exp_key_pair = KeyPair::new().unwrap();
        let auth_info = AuthInfo::RpoFalcon512(exp_key_pair);
        let bytes = auth_info.to_bytes();
        let actual = AuthInfo::read_from_bytes(&bytes).unwrap();
        match actual {
            AuthInfo::RpoFalcon512(act_key_pair) => {
                assert_eq!(exp_key_pair, act_key_pair)
            }
        }
    }

    #[test]
    fn test_account_code_insertion_no_duplicates() {
        let mut store = create_test_store();
        let assembler = assembler();
        let account_code = account::mock_account_code(&assembler);
        let tx = store.db.transaction().unwrap();

        // Table is empty at the beginning
        let mut actual: usize = tx
            .query_row("SELECT Count(*) FROM account_code", [], |row| row.get(0))
            .unwrap();
        assert_eq!(actual, 0);

        // First insertion generates a new row
        store::Store::insert_account_code(&tx, &account_code).unwrap();
        actual = tx
            .query_row("SELECT Count(*) FROM account_code", [], |row| row.get(0))
            .unwrap();
        assert_eq!(actual, 1);

        // Second insertion does not generate a new row
        actual = tx
            .query_row("SELECT Count(*) FROM account_code", [], |row| row.get(0))
            .unwrap();
        assert_eq!(actual, 1);
    }
}

use super::Store;

use crate::errors::StoreError;

use clap::error::Result;
use crypto::hash::rpo::RpoDigest;
use crypto::utils::{Deserializable, Serializable};
use crypto::{dsa::rpo_falcon512::KeyPair, utils::collections::BTreeMap, Word};
use objects::accounts::AccountStub;
use objects::assembly::AstSerdeOptions;
use objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountVault},
    assembly::ModuleAst,
    assets::Asset,
    Digest,
};
use rusqlite::{params, Transaction};

// TYPES
// ================================================================================================
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

impl Store {
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
                    .map(|id: u64| AccountId::try_from(id).expect("account id is valid"))
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

        const QUERY: &str = "INSERT INTO account_code (root, procedures, module) VALUES (?, ?, ?)";
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
        const QUERY: &str = "INSERT INTO account_vaults (root, assets) VALUES (?, ?)";
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

#[cfg(test)]
pub mod tests {
    use crypto::{
        dsa::rpo_falcon512::KeyPair,
        utils::{Deserializable, Serializable},
    };
    use miden_lib::assembler::assembler;
    use mock::mock::account;

    use crate::store::{self, tests::create_test_store};

    use super::AuthInfo;

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
        assert!(store::Store::insert_account_code(&tx, &account_code).is_err());
        actual = tx
            .query_row("SELECT Count(*) FROM account_code", [], |row| row.get(0))
            .unwrap();
        assert_eq!(actual, 1);
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
}

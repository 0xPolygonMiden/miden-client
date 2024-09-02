use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountStub, AuthSecretKey},
    assets::{Asset, AssetVault},
    Digest, Felt, Word,
};
use miden_tx::utils::{Deserializable, Serializable};
use rusqlite::{params, Transaction};

use super::SqliteStore;
use crate::store::StoreError;

// TYPES
// ================================================================================================
type SerializedAccountData = (i64, String, String, String, i64, bool, String);
type SerializedAccountsParts = (i64, i64, String, String, String, Option<Vec<u8>>);

type SerializedAccountAuthData = (i64, Vec<u8>, Vec<u8>);
type SerializedAccountAuthParts = (i64, Vec<u8>);

type SerializedAccountVaultData = (String, String);

type SerializedAccountCodeData = (String, Vec<u8>);

type SerializedAccountStorageData = (String, Vec<u8>);

type SerializedFullAccountParts = (i64, i64, Option<Vec<u8>>, Vec<u8>, Vec<u8>, String);

impl SqliteStore {
    // ACCOUNTS
    // --------------------------------------------------------------------------------------------

    pub(super) fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        const QUERY: &str = "SELECT DISTINCT id FROM accounts";

        self.db()
            .prepare(QUERY)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                Ok(result
                    .map(|id: i64| AccountId::try_from(id as u64).expect("account id is valid"))?)
            })
            .collect::<Result<Vec<AccountId>, StoreError>>()
    }

    pub(super) fn get_account_stubs(&self) -> Result<Vec<(AccountStub, Option<Word>)>, StoreError> {
        const QUERY: &str =
            "SELECT a.id, a.nonce, a.vault_root, a.storage_root, a.code_root, a.account_seed \
            FROM accounts a \
            WHERE a.nonce = (SELECT MAX(b.nonce) FROM accounts b WHERE b.id = a.id)";

        self.db()
            .prepare(QUERY)?
            .query_map([], parse_accounts_columns)
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).and_then(parse_accounts))
            .collect()
    }

    pub(crate) fn get_account_stub(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), StoreError> {
        let account_id_int: u64 = account_id.into();
        const QUERY: &str = "SELECT id, nonce, vault_root, storage_root, code_root, account_seed \
            FROM accounts WHERE id = ? \
            ORDER BY nonce DESC \
            LIMIT 1";
        self.db()
            .prepare(QUERY)?
            .query_map(params![account_id_int as i64], parse_accounts_columns)?
            .map(|result| Ok(result?).and_then(parse_accounts))
            .next()
            .ok_or(StoreError::AccountDataNotFound(account_id))?
    }

    pub(crate) fn get_account_stub_by_hash(
        &self,
        account_hash: Digest,
    ) -> Result<Option<AccountStub>, StoreError> {
        let account_hash_str: String = account_hash.to_string();
        const QUERY: &str = "SELECT id, nonce, vault_root, storage_root, code_root, account_seed \
            FROM accounts WHERE account_hash = ?";

        self.db()
            .prepare(QUERY)?
            .query_map(params![account_hash_str], parse_accounts_columns)?
            .map(|result| {
                let result = result?;
                Ok(parse_accounts(result)?.0)
            })
            .next()
            .map_or(Ok(None), |result| result.map(Some))
    }

    pub(crate) fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<(Account, Option<Word>), StoreError> {
        let account_id_int: u64 = account_id.into();
        const QUERY: &str = "SELECT accounts.id, accounts.nonce, accounts.account_seed, account_code.code, account_storage.slots, account_vaults.assets \
                            FROM accounts \
                            JOIN account_code ON accounts.code_root = account_code.root \
                            JOIN account_storage ON accounts.storage_root = account_storage.root \
                            JOIN account_vaults ON accounts.vault_root = account_vaults.root \
                            WHERE accounts.id = ? \
                            ORDER BY accounts.nonce DESC \
                            LIMIT 1";

        let result = self
            .db()
            .prepare(QUERY)?
            .query_map(params![account_id_int as i64], parse_account_columns)?
            .map(|result| Ok(result?).and_then(parse_account))
            .next()
            .ok_or(StoreError::AccountDataNotFound(account_id))?;
        let (account, account_seed) = result?;

        Ok((account, account_seed))
    }

    /// Retrieve account keys data by Account Id
    pub(crate) fn get_account_auth(
        &self,
        account_id: AccountId,
    ) -> Result<AuthSecretKey, StoreError> {
        let account_id_int: u64 = account_id.into();
        const QUERY: &str = "SELECT account_id, auth_info FROM account_auth WHERE account_id = ?";
        self.db()
            .prepare(QUERY)?
            .query_map(params![account_id_int as i64], parse_account_auth_columns)?
            .map(|result| Ok(result?).and_then(parse_account_auth))
            .next()
            .ok_or(StoreError::AccountDataNotFound(account_id))?
    }

    pub(crate) fn insert_account(
        &self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthSecretKey,
    ) -> Result<(), StoreError> {
        let mut db = self.db();
        let tx = db.transaction()?;

        insert_account_code(&tx, account.code())?;
        insert_account_storage(&tx, account.storage())?;
        insert_account_asset_vault(&tx, account.vault())?;
        insert_account_record(&tx, account, account_seed)?;
        insert_account_auth(&tx, account.id(), auth_info)?;

        Ok(tx.commit()?)
    }

    /// Returns an [AuthSecretKey] by a public key represented by a [Word]
    pub fn get_account_auth_by_pub_key(&self, pub_key: Word) -> Result<AuthSecretKey, StoreError> {
        let pub_key_bytes = pub_key.to_bytes();
        const QUERY: &str = "SELECT account_id, auth_info FROM account_auth WHERE pub_key = ?";
        self.db()
            .prepare(QUERY)?
            .query_map(params![pub_key_bytes], parse_account_auth_columns)?
            .map(|result| Ok(result?).and_then(parse_account_auth))
            .next()
            .ok_or(StoreError::AccountKeyNotFound(pub_key))?
    }
}

// HELPERS
// ================================================================================================

/// Update previously-existing account after a transaction execution
///
/// Because the Client retrieves the account by account ID before applying the delta, we don't
/// need to check that it exists here. This inserts a new row into the accounts table.
/// We can later identify the proper account state by looking at the nonce.
pub(crate) fn update_account(
    tx: &Transaction<'_>,
    new_account_state: &Account,
) -> Result<(), StoreError> {
    insert_account_storage(tx, new_account_state.storage())?;
    insert_account_asset_vault(tx, new_account_state.vault())?;
    insert_account_record(tx, new_account_state, None)
}

pub(super) fn insert_account_record(
    tx: &Transaction<'_>,
    account: &Account,
    account_seed: Option<Word>,
) -> Result<(), StoreError> {
    let (id, code_root, storage_root, vault_root, nonce, committed, hash) =
        serialize_account(account)?;

    let account_seed = account_seed.map(|seed| seed.to_bytes());

    const QUERY: &str =  "INSERT INTO accounts (id, code_root, storage_root, vault_root, nonce, committed, account_seed, account_hash) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";
    tx.execute(
        QUERY,
        params![id, code_root, storage_root, vault_root, nonce, committed, account_seed, hash],
    )?;
    Ok(())
}

/// Inserts an [AccountCode]
fn insert_account_code(tx: &Transaction<'_>, account_code: &AccountCode) -> Result<(), StoreError> {
    let (code_root, code) = serialize_account_code(account_code)?;
    const QUERY: &str = "INSERT OR IGNORE INTO account_code (root, code) VALUES (?, ?)";
    tx.execute(QUERY, params![code_root, code,])?;
    Ok(())
}

/// Inserts an [AccountStorage]
pub(super) fn insert_account_storage(
    tx: &Transaction<'_>,
    account_storage: &AccountStorage,
) -> Result<(), StoreError> {
    let (storage_root, storage_slots) = serialize_account_storage(account_storage)?;
    const QUERY: &str = "INSERT OR IGNORE INTO account_storage (root, slots) VALUES (?, ?)";
    tx.execute(QUERY, params![storage_root, storage_slots])?;
    Ok(())
}

/// Inserts an [AssetVault]
pub(super) fn insert_account_asset_vault(
    tx: &Transaction<'_>,
    asset_vault: &AssetVault,
) -> Result<(), StoreError> {
    let (vault_root, assets) = serialize_account_asset_vault(asset_vault)?;
    const QUERY: &str = "INSERT OR IGNORE INTO account_vaults (root, assets) VALUES (?, ?)";
    tx.execute(QUERY, params![vault_root, assets])?;
    Ok(())
}

/// Inserts an [AuthSecretKey] for the account with id `account_id`
pub(super) fn insert_account_auth(
    tx: &Transaction<'_>,
    account_id: AccountId,
    auth_info: &AuthSecretKey,
) -> Result<(), StoreError> {
    let (account_id, auth_info, pub_key) = serialize_account_auth(account_id, auth_info)?;
    const QUERY: &str =
        "INSERT INTO account_auth (account_id, auth_info, pub_key) VALUES (?, ?, ?)";

    tx.execute(QUERY, params![account_id, auth_info, pub_key])?;
    Ok(())
}

/// Parse accounts colums from the provided row into native types
pub(super) fn parse_accounts_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedAccountsParts, rusqlite::Error> {
    let id: i64 = row.get(0)?;
    let nonce: i64 = row.get(1)?;
    let vault_root: String = row.get(2)?;
    let storage_root: String = row.get(3)?;
    let code_root: String = row.get(4)?;
    let account_seed: Option<Vec<u8>> = row.get(5)?;
    Ok((id, nonce, vault_root, storage_root, code_root, account_seed))
}

/// Parse an account from the provided parts.
pub(super) fn parse_accounts(
    serialized_account_parts: SerializedAccountsParts,
) -> Result<(AccountStub, Option<Word>), StoreError> {
    let (id, nonce, vault_root, storage_root, code_root, account_seed) = serialized_account_parts;
    let account_seed = account_seed.map(|seed| Word::read_from_bytes(&seed)).transpose()?;

    Ok((
        AccountStub::new(
            (id as u64)
                .try_into()
                .expect("Conversion from stored AccountID should not panic"),
            Felt::new(nonce as u64),
            Digest::try_from(&vault_root)?,
            Digest::try_from(&storage_root)?,
            Digest::try_from(&code_root)?,
        ),
        account_seed,
    ))
}

/// Parse an account from the provided parts.
pub(super) fn parse_account(
    serialized_account_parts: SerializedFullAccountParts,
) -> Result<(Account, Option<Word>), StoreError> {
    let (id, nonce, account_seed, code, storage, assets) = serialized_account_parts;
    let account_seed = account_seed.map(|seed| Word::read_from_bytes(&seed)).transpose()?;
    let account_id: AccountId = (id as u64)
        .try_into()
        .expect("Conversion from stored AccountID should not panic");
    let account_code = AccountCode::from_bytes(&code)?;
    let account_storage = AccountStorage::read_from_bytes(&storage)?;
    let account_assets: Vec<Asset> =
        serde_json::from_str(&assets).map_err(StoreError::JsonDataDeserializationError)?;

    Ok((
        Account::from_parts(
            account_id,
            AssetVault::new(&account_assets)?,
            account_storage,
            account_code,
            Felt::new(nonce as u64),
        ),
        account_seed,
    ))
}

/// Serialized the provided account into database compatible types.
fn serialize_account(account: &Account) -> Result<SerializedAccountData, StoreError> {
    let id: u64 = account.id().into();
    let code_root = account.code().commitment().to_string();
    let storage_root = account.storage().root().to_string();
    let vault_root = String::from(&account.vault().commitment());
    let committed = account.is_on_chain();
    let nonce = account.nonce().as_int() as i64;
    let hash = account.hash().to_string();

    Ok((id as i64, code_root, storage_root, vault_root, nonce, committed, hash))
}

/// Parse account_auth columns from the provided row into native types
fn parse_account_auth_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedAccountAuthParts, rusqlite::Error> {
    let account_id: i64 = row.get(0)?;
    let auth_info_bytes: Vec<u8> = row.get(1)?;
    Ok((account_id, auth_info_bytes))
}

/// Parse an `AuthSecretKey` from the provided parts.
fn parse_account_auth(
    serialized_account_auth_parts: SerializedAccountAuthParts,
) -> Result<AuthSecretKey, StoreError> {
    let (_, auth_info_bytes) = serialized_account_auth_parts;
    let auth_info = AuthSecretKey::read_from_bytes(&auth_info_bytes)?;
    Ok(auth_info)
}

/// Serialized the provided account_auth into database compatible types.
fn serialize_account_auth(
    account_id: AccountId,
    auth_info: &AuthSecretKey,
) -> Result<SerializedAccountAuthData, StoreError> {
    let pub_key = match auth_info {
        AuthSecretKey::RpoFalcon512(secret) => Word::from(secret.public_key()),
    }
    .to_bytes();

    let account_id: u64 = account_id.into();
    let auth_info = auth_info.to_bytes();

    Ok((account_id as i64, auth_info, pub_key))
}

/// Serialize the provided account_code into database compatible types.
fn serialize_account_code(
    account_code: &AccountCode,
) -> Result<SerializedAccountCodeData, StoreError> {
    let root = account_code.commitment().to_string();
    let code = account_code.to_bytes();

    Ok((root, code))
}

/// Serialize the provided account_storage into database compatible types.
fn serialize_account_storage(
    account_storage: &AccountStorage,
) -> Result<SerializedAccountStorageData, StoreError> {
    let root = account_storage.root().to_string();
    let storage = account_storage.to_bytes();

    Ok((root, storage))
}

/// Serialize the provided asset_vault into database compatible types.
fn serialize_account_asset_vault(
    asset_vault: &AssetVault,
) -> Result<SerializedAccountVaultData, StoreError> {
    let root = String::from(&asset_vault.commitment());
    let assets: Vec<Asset> = asset_vault.assets().collect();
    let assets = serde_json::to_string(&assets).map_err(StoreError::InputSerializationError)?;
    Ok((root, assets))
}

/// Parse accounts parts from the provided row into native types
pub(super) fn parse_account_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedFullAccountParts, rusqlite::Error> {
    let id: i64 = row.get(0)?;
    let nonce: i64 = row.get(1)?;
    let account_seed: Option<Vec<u8>> = row.get(2)?;
    let code: Vec<u8> = row.get(3)?;
    let storage: Vec<u8> = row.get(4)?;
    let assets: String = row.get(5)?;
    Ok((id, nonce, account_seed, code, storage, assets))
}

#[cfg(test)]
mod tests {
    use miden_objects::{
        accounts::{AccountCode, AccountId},
        crypto::dsa::rpo_falcon512::SecretKey,
    };
    use miden_tx::utils::{Deserializable, Serializable};

    use super::{insert_account_auth, AuthSecretKey};
    use crate::{
        mock::DEFAULT_ACCOUNT_CODE,
        store::sqlite_store::{accounts::insert_account_code, tests::create_test_store},
    };

    #[test]
    fn test_account_code_insertion_no_duplicates() {
        let store = create_test_store();
        let assembler = miden_lib::transaction::TransactionKernel::assembler();
        let account_code = AccountCode::compile(DEFAULT_ACCOUNT_CODE, assembler).unwrap();
        let mut db = store.db();
        let tx = db.transaction().unwrap();

        // Table is empty at the beginning
        let mut actual: usize =
            tx.query_row("SELECT Count(*) FROM account_code", [], |row| row.get(0)).unwrap();
        assert_eq!(actual, 0);

        // First insertion generates a new row
        insert_account_code(&tx, &account_code).unwrap();
        actual = tx.query_row("SELECT Count(*) FROM account_code", [], |row| row.get(0)).unwrap();
        assert_eq!(actual, 1);

        // Second insertion passes but does not generate a new row
        assert!(insert_account_code(&tx, &account_code).is_ok());
        actual = tx.query_row("SELECT Count(*) FROM account_code", [], |row| row.get(0)).unwrap();
        assert_eq!(actual, 1);
    }

    #[test]
    fn test_auth_info_serialization() {
        let exp_key_pair = SecretKey::new();
        let auth_info = AuthSecretKey::RpoFalcon512(exp_key_pair.clone());
        let bytes = auth_info.to_bytes();
        let actual = AuthSecretKey::read_from_bytes(&bytes).unwrap();
        match actual {
            AuthSecretKey::RpoFalcon512(act_key_pair) => {
                assert_eq!(exp_key_pair.to_bytes(), act_key_pair.to_bytes());
                assert_eq!(exp_key_pair.public_key(), act_key_pair.public_key());
            },
        }
    }

    #[test]
    fn test_auth_info_store() {
        let exp_key_pair = SecretKey::new();

        let store = create_test_store();

        let account_id = AccountId::try_from(3238098370154045919u64).unwrap();
        {
            let mut db = store.db();
            let tx = db.transaction().unwrap();
            insert_account_auth(
                &tx,
                account_id,
                &AuthSecretKey::RpoFalcon512(exp_key_pair.clone()),
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let account_auth = store.get_account_auth(account_id).unwrap();
        match account_auth {
            AuthSecretKey::RpoFalcon512(act_key_pair) => {
                assert_eq!(exp_key_pair.to_bytes(), act_key_pair.to_bytes());
                assert_eq!(exp_key_pair.public_key(), act_key_pair.public_key());
            },
        }
    }
}

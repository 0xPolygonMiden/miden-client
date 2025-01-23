use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use std::{collections::BTreeMap, rc::Rc};

use miden_objects::{
    account::{Account, AccountCode, AccountHeader, AccountId, AccountStorage, AuthSecretKey},
    asset::{Asset, AssetVault},
    AccountError, Digest, Felt, Word,
};
use miden_tx::utils::{Deserializable, Serializable};
use rusqlite::{params, types::Value, Connection, Transaction};

use super::SqliteStore;
use crate::store::{AccountRecord, AccountStatus, StoreError};

// TYPES
// ================================================================================================
type SerializedAccountData = (String, String, String, String, i64, bool, String);
type SerializedAccountsParts = (String, i64, String, String, String, Option<Vec<u8>>, bool);

type SerializedAccountAuthData = (String, Vec<u8>, Vec<u8>);
type SerializedAccountAuthParts = (String, Vec<u8>);

type SerializedAccountVaultData = (String, Vec<u8>);

type SerializedAccountCodeData = (String, Vec<u8>);

type SerializedAccountStorageData = (String, Vec<u8>);

type SerializedFullAccountParts = (String, i64, Option<Vec<u8>>, Vec<u8>, Vec<u8>, Vec<u8>, bool);

impl SqliteStore {
    // ACCOUNTS
    // --------------------------------------------------------------------------------------------

    pub(super) fn get_account_ids(conn: &mut Connection) -> Result<Vec<AccountId>, StoreError> {
        const QUERY: &str = "SELECT DISTINCT id FROM accounts";

        conn.prepare(QUERY)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                Ok(result
                    .map(|id: String| AccountId::from_hex(&id).expect("account id is valid"))?)
            })
            .collect::<Result<Vec<AccountId>, StoreError>>()
    }

    pub(super) fn get_account_headers(
        conn: &mut Connection,
    ) -> Result<Vec<(AccountHeader, AccountStatus)>, StoreError> {
        const QUERY: &str =
            "SELECT a.id, a.nonce, a.vault_root, a.storage_root, a.code_root, a.account_seed, a.locked \
            FROM accounts a \
            WHERE a.nonce = (SELECT MAX(b.nonce) FROM accounts b WHERE b.id = a.id)";

        conn.prepare(QUERY)?
            .query_map([], parse_accounts_columns)
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).and_then(parse_accounts))
            .collect()
    }

    pub(crate) fn get_account_header(
        conn: &mut Connection,
        account_id: AccountId,
    ) -> Result<Option<(AccountHeader, AccountStatus)>, StoreError> {
        const QUERY: &str =
            "SELECT id, nonce, vault_root, storage_root, code_root, account_seed, locked \
            FROM accounts WHERE id = ? \
            ORDER BY nonce DESC \
            LIMIT 1";
        conn.prepare(QUERY)?
            .query_map(params![account_id.to_hex()], parse_accounts_columns)?
            .map(|result| Ok(result?).and_then(parse_accounts))
            .next()
            .transpose()
    }

    pub(crate) fn get_account_header_by_hash(
        conn: &mut Connection,
        account_hash: Digest,
    ) -> Result<Option<AccountHeader>, StoreError> {
        let account_hash_str: String = account_hash.to_string();
        const QUERY: &str =
            "SELECT id, nonce, vault_root, storage_root, code_root, account_seed, locked \
            FROM accounts WHERE account_hash = ?";

        conn.prepare(QUERY)?
            .query_map(params![account_hash_str], parse_accounts_columns)?
            .map(|result| {
                let result = result?;
                Ok(parse_accounts(result)?.0)
            })
            .next()
            .map_or(Ok(None), |result| result.map(Some))
    }

    pub(crate) fn get_account(
        conn: &mut Connection,
        account_id: AccountId,
    ) -> Result<Option<AccountRecord>, StoreError> {
        const QUERY: &str = "SELECT accounts.id, accounts.nonce, accounts.account_seed, account_code.code, account_storage.slots, account_vaults.assets, accounts.locked \
                            FROM accounts \
                            JOIN account_code ON accounts.code_root = account_code.root \
                            JOIN account_storage ON accounts.storage_root = account_storage.root \
                            JOIN account_vaults ON accounts.vault_root = account_vaults.root \
                            WHERE accounts.id = ? \
                            ORDER BY accounts.nonce DESC \
                            LIMIT 1";

        conn.prepare(QUERY)?
            .query_map(params![account_id.to_hex()], parse_account_columns)?
            .map(|result| Ok(result?).and_then(parse_account))
            .next()
            .transpose()
    }

    /// Retrieve account keys data by Account ID.
    pub(crate) fn get_account_auth(
        conn: &mut Connection,
        account_id: AccountId,
    ) -> Result<Option<AuthSecretKey>, StoreError> {
        const QUERY: &str = "SELECT account_id, auth_info FROM account_auth WHERE account_id = ?";
        conn.prepare(QUERY)?
            .query_map(params![account_id.to_hex()], parse_account_auth_columns)?
            .map(|result| Ok(result?).and_then(parse_account_auth))
            .next()
            .transpose()
    }

    pub(crate) fn insert_account(
        conn: &mut Connection,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthSecretKey,
    ) -> Result<(), StoreError> {
        let tx = conn.transaction()?;

        insert_account_code(&tx, account.code())?;
        insert_account_storage(&tx, account.storage())?;
        insert_account_asset_vault(&tx, account.vault())?;
        insert_account_record(&tx, account, account_seed)?;
        insert_account_auth(&tx, account.id(), auth_info)?;

        Ok(tx.commit()?)
    }

    pub(crate) fn update_account(
        conn: &mut Connection,
        new_account_state: &Account,
    ) -> Result<(), StoreError> {
        const QUERY: &str = "SELECT id FROM accounts WHERE id = ?";
        if conn
            .prepare(QUERY)?
            .query_map(params![new_account_state.id().to_hex()], parse_account_auth_columns)?
            .map(|result| Ok(result?).and_then(parse_account_auth))
            .next()
            .is_none()
        {
            return Err(StoreError::AccountDataNotFound(new_account_state.id()));
        }

        let tx = conn.transaction()?;
        update_account(&tx, new_account_state)?;
        Ok(tx.commit()?)
    }

    /// Returns an [AuthSecretKey] by a public key represented by a [Word].
    pub fn get_account_auth_by_pub_key(
        conn: &mut Connection,
        pub_key: Word,
    ) -> Result<Option<AuthSecretKey>, StoreError> {
        let pub_key_bytes = pub_key.to_bytes();
        const QUERY: &str = "SELECT account_id, auth_info FROM account_auth WHERE pub_key = ?";
        conn.prepare(QUERY)?
            .query_map(params![pub_key_bytes], parse_account_auth_columns)?
            .map(|result| Ok(result?).and_then(parse_account_auth))
            .next()
            .transpose()
    }

    pub fn upsert_foreign_account_code(
        conn: &mut Connection,
        account_id: AccountId,
        code: AccountCode,
    ) -> Result<(), StoreError> {
        let tx = conn.transaction()?;

        const QUERY: &str =
            "INSERT OR REPLACE INTO foreign_account_code (account_id, code_root) VALUES (?, ?)";
        tx.execute(QUERY, params![account_id.to_hex(), code.commitment().to_string()])?;

        insert_account_code(&tx, &code)?;
        Ok(tx.commit()?)
    }

    pub fn get_foreign_account_code(
        conn: &mut Connection,
        account_ids: Vec<AccountId>,
    ) -> Result<BTreeMap<AccountId, AccountCode>, StoreError> {
        let params: Vec<Value> =
            account_ids.into_iter().map(|id| Value::from(id.to_hex())).collect();
        const QUERY: &str = "
            SELECT account_id, code
            FROM foreign_account_code JOIN account_code ON code_root = code_root
            WHERE account_id IN rarray(?)";

        conn.prepare(QUERY)?
            .query_map([Rc::new(params)], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("no binding parameters used in query")
            .map(|result| {
                result.map_err(|err| StoreError::ParsingError(err.to_string())).and_then(
                    |(id, code): (String, Vec<u8>)| {
                        Ok((
                            AccountId::from_hex(&id).map_err(|err| {
                                StoreError::AccountError(
                                    AccountError::FinalAccountHeaderIdParsingFailed(err),
                                )
                            })?,
                            AccountCode::from_bytes(&code).map_err(StoreError::AccountError)?,
                        ))
                    },
                )
            })
            .collect::<Result<BTreeMap<AccountId, AccountCode>, _>>()
    }
}

// HELPERS
// ================================================================================================

/// Update previously-existing account after a transaction execution.
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

    const QUERY: &str =  "INSERT OR REPLACE INTO accounts (id, code_root, storage_root, vault_root, nonce, committed, account_seed, account_hash, locked) VALUES (?, ?, ?, ?, ?, ?, ?, ?, false)";
    tx.execute(
        QUERY,
        params![id, code_root, storage_root, vault_root, nonce, committed, account_seed, hash],
    )?;
    Ok(())
}

/// Inserts an [AccountCode].
fn insert_account_code(tx: &Transaction<'_>, account_code: &AccountCode) -> Result<(), StoreError> {
    let (code_root, code) = serialize_account_code(account_code)?;
    const QUERY: &str = "INSERT OR IGNORE INTO account_code (root, code) VALUES (?, ?)";
    tx.execute(QUERY, params![code_root, code,])?;
    Ok(())
}

/// Inserts an [AccountStorage].
pub(super) fn insert_account_storage(
    tx: &Transaction<'_>,
    account_storage: &AccountStorage,
) -> Result<(), StoreError> {
    let (storage_root, storage_slots) = serialize_account_storage(account_storage)?;
    const QUERY: &str = "INSERT OR IGNORE INTO account_storage (root, slots) VALUES (?, ?)";
    tx.execute(QUERY, params![storage_root, storage_slots])?;
    Ok(())
}

/// Inserts an [AssetVault].
pub(super) fn insert_account_asset_vault(
    tx: &Transaction<'_>,
    asset_vault: &AssetVault,
) -> Result<(), StoreError> {
    let (vault_root, assets) = serialize_account_asset_vault(asset_vault)?;
    const QUERY: &str = "INSERT OR IGNORE INTO account_vaults (root, assets) VALUES (?, ?)";
    tx.execute(QUERY, params![vault_root, assets])?;
    Ok(())
}

/// Inserts an [AuthSecretKey] for the account with ID `account_id`.
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

pub(super) fn lock_account(tx: &Transaction<'_>, account_id: AccountId) -> Result<(), StoreError> {
    const QUERY: &str = "UPDATE accounts SET locked = true WHERE id = ?";
    tx.execute(QUERY, params![account_id.to_hex()])?;
    Ok(())
}

/// Parse accounts columns from the provided row into native types.
pub(super) fn parse_accounts_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedAccountsParts, rusqlite::Error> {
    let id: String = row.get(0)?;
    let nonce: i64 = row.get(1)?;
    let vault_root: String = row.get(2)?;
    let storage_root: String = row.get(3)?;
    let code_root: String = row.get(4)?;
    let account_seed: Option<Vec<u8>> = row.get(5)?;
    let locked: bool = row.get(6)?;
    Ok((id, nonce, vault_root, storage_root, code_root, account_seed, locked))
}

/// Parse an account from the provided parts.
pub(super) fn parse_accounts(
    serialized_account_parts: SerializedAccountsParts,
) -> Result<(AccountHeader, AccountStatus), StoreError> {
    let (id, nonce, vault_root, storage_root, code_root, account_seed, locked) =
        serialized_account_parts;
    let account_seed = account_seed.map(|seed| Word::read_from_bytes(&seed)).transpose()?;

    let status = match (account_seed, locked) {
        (_, true) => AccountStatus::Locked,
        (Some(seed), _) => AccountStatus::New { seed },
        _ => AccountStatus::Tracked,
    };

    Ok((
        AccountHeader::new(
            AccountId::from_hex(&id).expect("Conversion from stored AccountID should not panic"),
            Felt::new(nonce as u64),
            Digest::try_from(&vault_root)?,
            Digest::try_from(&storage_root)?,
            Digest::try_from(&code_root)?,
        ),
        status,
    ))
}

/// Parse an account from the provided parts.
pub(super) fn parse_account(
    serialized_account_parts: SerializedFullAccountParts,
) -> Result<AccountRecord, StoreError> {
    let (id, nonce, account_seed, code, storage, assets, locked) = serialized_account_parts;
    let account_seed = account_seed.map(|seed| Word::read_from_bytes(&seed)).transpose()?;
    let account_id: AccountId =
        AccountId::from_hex(&id).expect("Conversion from stored AccountID should not panic");
    let account_code = AccountCode::from_bytes(&code)?;
    let account_storage = AccountStorage::read_from_bytes(&storage)?;
    let account_assets: Vec<Asset> = Vec::<Asset>::read_from_bytes(&assets)?;
    let account = Account::from_parts(
        account_id,
        AssetVault::new(&account_assets)?,
        account_storage,
        account_code,
        Felt::new(nonce as u64),
    );

    let status = match (account_seed, locked) {
        (_, true) => AccountStatus::Locked,
        (Some(seed), _) => AccountStatus::New { seed },
        _ => AccountStatus::Tracked,
    };

    Ok(AccountRecord::new(account, status))
}

/// Serialized the provided account into database compatible types.
fn serialize_account(account: &Account) -> Result<SerializedAccountData, StoreError> {
    let id: String = account.id().to_hex();
    let code_root = account.code().commitment().to_string();
    let commitment_root = account.storage().commitment().to_string();
    let vault_root = account.vault().commitment().to_string();
    let committed = account.is_public();
    let nonce = account.nonce().as_int() as i64;
    let hash = account.hash().to_string();

    Ok((id, code_root, commitment_root, vault_root, nonce, committed, hash))
}

/// Parse account_auth columns from the provided row into native types.
fn parse_account_auth_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedAccountAuthParts, rusqlite::Error> {
    let account_id: String = row.get(0)?;
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

    let account_id: String = account_id.to_hex();
    let auth_info = auth_info.to_bytes();

    Ok((account_id, auth_info, pub_key))
}

/// Serialize the provided account_code into database compatible types.
fn serialize_account_code(
    account_code: &AccountCode,
) -> Result<SerializedAccountCodeData, StoreError> {
    let commitment = account_code.commitment().to_string();
    let code = account_code.to_bytes();

    Ok((commitment, code))
}

/// Serialize the provided account_storage into database compatible types.
fn serialize_account_storage(
    account_storage: &AccountStorage,
) -> Result<SerializedAccountStorageData, StoreError> {
    let commitment = account_storage.commitment().to_string();
    let storage = account_storage.to_bytes();

    Ok((commitment, storage))
}

/// Serialize the provided asset_vault into database compatible types.
fn serialize_account_asset_vault(
    asset_vault: &AssetVault,
) -> Result<SerializedAccountVaultData, StoreError> {
    let commitment = asset_vault.commitment().to_string();
    let assets = asset_vault.assets().collect::<Vec<Asset>>().to_bytes();
    Ok((commitment, assets))
}

/// Parse accounts parts from the provided row into native types.
pub(super) fn parse_account_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedFullAccountParts, rusqlite::Error> {
    let id: String = row.get(0)?;
    let nonce: i64 = row.get(1)?;
    let account_seed: Option<Vec<u8>> = row.get(2)?;
    let code: Vec<u8> = row.get(3)?;
    let storage: Vec<u8> = row.get(4)?;
    let assets: Vec<u8> = row.get(5)?;
    let locked: bool = row.get(6)?;

    Ok((id, nonce, account_seed, code, storage, assets, locked))
}

#[cfg(test)]
mod tests {
    use miden_objects::{
        account::{AccountCode, AccountComponent, AccountId},
        crypto::dsa::rpo_falcon512::SecretKey,
        testing::{
            account_component::BASIC_WALLET_CODE,
            account_id::ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        },
    };
    use miden_tx::utils::{Deserializable, Serializable};

    use super::{insert_account_auth, AuthSecretKey};
    use crate::store::{
        sqlite_store::{accounts::insert_account_code, tests::create_test_store},
        Store,
    };

    #[tokio::test]
    async fn test_account_code_insertion_no_duplicates() {
        let store = create_test_store().await;
        let assembler = miden_lib::transaction::TransactionKernel::assembler();
        let account_component = AccountComponent::compile(BASIC_WALLET_CODE, assembler, vec![])
            .unwrap()
            .with_supports_all_types();
        let account_code = AccountCode::from_components(
            &[account_component],
            miden_objects::account::AccountType::RegularAccountUpdatableCode,
        )
        .unwrap();
        store
            .interact_with_connection(move |conn| {
                let tx = conn.transaction().unwrap();

                // Table is empty at the beginning
                let mut actual: usize = tx
                    .query_row("SELECT Count(*) FROM account_code", [], |row| row.get(0))
                    .unwrap();
                assert_eq!(actual, 0);

                // First insertion generates a new row
                insert_account_code(&tx, &account_code).unwrap();
                actual = tx
                    .query_row("SELECT Count(*) FROM account_code", [], |row| row.get(0))
                    .unwrap();
                assert_eq!(actual, 1);

                // Second insertion passes but does not generate a new row
                assert!(insert_account_code(&tx, &account_code).is_ok());
                actual = tx
                    .query_row("SELECT Count(*) FROM account_code", [], |row| row.get(0))
                    .unwrap();
                assert_eq!(actual, 1);

                Ok(())
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_auth_info_serialization() {
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

    #[tokio::test]
    async fn test_auth_info_store() {
        let exp_key_pair = SecretKey::new();

        let store = create_test_store().await;

        let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN)
            .expect("account id is valid");
        {
            let exp_key_pair_clone = exp_key_pair.clone();
            store
                .interact_with_connection(move |conn| {
                    let tx = conn.transaction().unwrap();
                    insert_account_auth(
                        &tx,
                        account_id,
                        &AuthSecretKey::RpoFalcon512(exp_key_pair_clone),
                    )
                    .unwrap();
                    tx.commit().unwrap();
                    Ok(())
                })
                .await
                .unwrap();
        }

        let account_auth = Store::get_account_auth(&store, account_id).await.unwrap().unwrap();

        match account_auth {
            AuthSecretKey::RpoFalcon512(act_key_pair) => {
                assert_eq!(exp_key_pair.to_bytes(), act_key_pair.to_bytes());
                assert_eq!(exp_key_pair.public_key(), act_key_pair.public_key());
            },
        }
    }
}

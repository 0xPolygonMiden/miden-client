#![allow(clippy::items_after_statements)]

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use std::{collections::BTreeMap, rc::Rc};

use miden_objects::{
    AccountError, Digest, Felt, Word,
    account::{Account, AccountCode, AccountHeader, AccountId, AccountStorage},
    asset::{Asset, AssetVault},
};
use miden_tx::utils::{Deserializable, Serializable};
use rusqlite::{Connection, Transaction, named_params, params, types::Value};

use super::{SqliteStore, column_value_as_u64, u64_to_value};
use crate::{
    insert_sql,
    store::{AccountRecord, AccountStatus, StoreError},
    subst,
};

// TYPES
// ================================================================================================
type SerializedAccountData = (String, String, String, String, Value, bool, String);
type SerializedAccountsParts = (String, u64, String, String, String, Option<Vec<u8>>, bool);

type SerializedAccountVaultData = (String, Vec<u8>);

type SerializedAccountCodeData = (String, Vec<u8>);

type SerializedAccountStorageData = (String, Vec<u8>);

type SerializedFullAccountParts = (String, u64, Option<Vec<u8>>, Vec<u8>, Vec<u8>, Vec<u8>, bool);

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
        const QUERY: &str = "SELECT a.id, a.nonce, a.vault_root, a.storage_root, a.code_root, a.account_seed, a.locked \
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
        const QUERY: &str = "SELECT id, nonce, vault_root, storage_root, code_root, account_seed, locked \
            FROM accounts WHERE id = ? \
            ORDER BY nonce DESC \
            LIMIT 1";
        conn.prepare(QUERY)?
            .query_map(params![account_id.to_hex()], parse_accounts_columns)?
            .map(|result| Ok(result?).and_then(parse_accounts))
            .next()
            .transpose()
    }

    pub(crate) fn get_account_header_by_commitment(
        conn: &mut Connection,
        account_commitment: Digest,
    ) -> Result<Option<AccountHeader>, StoreError> {
        let account_commitment_str: String = account_commitment.to_string();
        const QUERY: &str = "SELECT id, nonce, vault_root, storage_root, code_root, account_seed, locked \
            FROM accounts WHERE account_commitment = ?";

        conn.prepare(QUERY)?
            .query_map(params![account_commitment_str], parse_accounts_columns)?
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

    pub(crate) fn insert_account(
        conn: &mut Connection,
        account: &Account,
        account_seed: Option<Word>,
    ) -> Result<(), StoreError> {
        let tx = conn.transaction()?;

        insert_account_code(&tx, account.code())?;
        insert_account_storage(&tx, account.storage())?;
        insert_account_asset_vault(&tx, account.vault())?;
        insert_account_record(&tx, account, account_seed)?;

        Ok(tx.commit()?)
    }

    pub(crate) fn update_account(
        conn: &mut Connection,
        new_account_state: &Account,
    ) -> Result<(), StoreError> {
        const QUERY: &str = "SELECT id FROM accounts WHERE id = ?";
        if conn
            .prepare(QUERY)?
            .query_map(params![new_account_state.id().to_hex()], |row| row.get(0))?
            .map(|result| {
                result.map_err(|err| StoreError::ParsingError(err.to_string())).and_then(
                    |id: String| {
                        AccountId::from_hex(&id).map_err(|err| {
                            StoreError::AccountError(
                                AccountError::FinalAccountHeaderIdParsingFailed(err),
                            )
                        })
                    },
                )
            })
            .next()
            .is_none()
        {
            return Err(StoreError::AccountDataNotFound(new_account_state.id()));
        }

        let tx = conn.transaction()?;
        update_account(&tx, new_account_state)?;
        Ok(tx.commit()?)
    }

    pub fn upsert_foreign_account_code(
        conn: &mut Connection,
        account_id: AccountId,
        code: &AccountCode,
    ) -> Result<(), StoreError> {
        let tx = conn.transaction()?;

        const QUERY: &str = insert_sql!(foreign_account_code { account_id, code_root } | REPLACE);

        tx.execute(QUERY, params![account_id.to_hex(), code.commitment().to_string()])?;

        insert_account_code(&tx, code)?;
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
    let (id, code_root, storage_root, vault_root, nonce, committed, commitment) =
        serialize_account(account);

    let account_seed = account_seed.map(|seed| seed.to_bytes());

    const QUERY: &str = insert_sql!(
        accounts {
            id,
            code_root,
            storage_root,
            vault_root,
            nonce,
            committed,
            account_seed,
            account_commitment,
            locked
        } | REPLACE
    );

    tx.execute(
        QUERY,
        params![
            id,
            code_root,
            storage_root,
            vault_root,
            nonce,
            committed,
            account_seed,
            commitment,
            false,
        ],
    )?;
    Ok(())
}

/// Inserts an [`AccountCode`].
fn insert_account_code(tx: &Transaction<'_>, account_code: &AccountCode) -> Result<(), StoreError> {
    let (code_root, code) = serialize_account_code(account_code);
    const QUERY: &str = insert_sql!(account_code { root, code } | IGNORE);
    tx.execute(QUERY, params![code_root, code])?;
    Ok(())
}

/// Inserts an [`AccountStorage`].
pub(super) fn insert_account_storage(
    tx: &Transaction<'_>,
    account_storage: &AccountStorage,
) -> Result<(), StoreError> {
    let (storage_root, storage_slots) = serialize_account_storage(account_storage);
    const QUERY: &str = insert_sql!(account_storage { root, slots } | IGNORE);
    tx.execute(QUERY, params![storage_root, storage_slots])?;
    Ok(())
}

/// Inserts an [`AssetVault`].
pub(super) fn insert_account_asset_vault(
    tx: &Transaction<'_>,
    asset_vault: &AssetVault,
) -> Result<(), StoreError> {
    let (vault_root, assets) = serialize_account_asset_vault(asset_vault);
    const QUERY: &str = insert_sql!(account_vaults { root, assets } | IGNORE);
    tx.execute(QUERY, params![vault_root, assets])?;
    Ok(())
}

/// Locks the account if the mismatched digest doesn't belong to a previous account state (stale
/// data).
pub(super) fn lock_account_on_unexpected_commitment(
    tx: &Transaction<'_>,
    account_id: &AccountId,
    mismatched_digest: &Digest,
) -> Result<(), StoreError> {
    // Mismatched digests may be due to stale network data. If the mismatched digest is
    // tracked in the db and corresponds to the mismatched account, it means we
    // got a past update and shouldn't lock the account.
    const QUERY: &str = "UPDATE accounts SET locked = true WHERE id = :account_id AND NOT EXISTS (SELECT 1 FROM accounts WHERE id = :account_id AND account_commitment = :digest)";
    tx.execute(
        QUERY,
        named_params! {
            ":account_id": account_id.to_hex(),
            ":digest": mismatched_digest.to_string()
        },
    )?;
    Ok(())
}

/// Parse accounts columns from the provided row into native types.
pub(super) fn parse_accounts_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedAccountsParts, rusqlite::Error> {
    let id: String = row.get(0)?;
    let nonce: u64 = column_value_as_u64(row, 1)?;
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
            Felt::new(nonce),
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
        Felt::new(nonce),
    );

    let status = match (account_seed, locked) {
        (_, true) => AccountStatus::Locked,
        (Some(seed), _) => AccountStatus::New { seed },
        _ => AccountStatus::Tracked,
    };

    Ok(AccountRecord::new(account, status))
}

/// Serialized the provided account into database compatible types.
// TODO: review the clippy exemption.
fn serialize_account(account: &Account) -> SerializedAccountData {
    let id: String = account.id().to_hex();
    let code_root = account.code().commitment().to_string();
    let commitment_root = account.storage().commitment().to_string();
    let vault_root = account.vault().root().to_string();
    let committed = account.is_public();
    let nonce = u64_to_value(account.nonce().as_int());
    let commitment = account.commitment().to_string();

    (id, code_root, commitment_root, vault_root, nonce, committed, commitment)
}

/// Serialize the provided `account_code` into database compatible types.
fn serialize_account_code(account_code: &AccountCode) -> SerializedAccountCodeData {
    let commitment = account_code.commitment().to_string();
    let code = account_code.to_bytes();

    (commitment, code)
}

/// Serialize the provided `account_storage` into database compatible types.
fn serialize_account_storage(account_storage: &AccountStorage) -> SerializedAccountStorageData {
    let commitment = account_storage.commitment().to_string();
    let storage = account_storage.to_bytes();

    (commitment, storage)
}

/// Serialize the provided `asset_vault` into database compatible types.
fn serialize_account_asset_vault(asset_vault: &AssetVault) -> SerializedAccountVaultData {
    let commitment = asset_vault.root().to_string();
    let assets = asset_vault.assets().collect::<Vec<Asset>>().to_bytes();
    (commitment, assets)
}

/// Parse accounts parts from the provided row into native types.
pub(super) fn parse_account_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedFullAccountParts, rusqlite::Error> {
    let id: String = row.get(0)?;
    let nonce: u64 = column_value_as_u64(row, 1)?;
    let account_seed: Option<Vec<u8>> = row.get(2)?;
    let code: Vec<u8> = row.get(3)?;
    let storage: Vec<u8> = row.get(4)?;
    let assets: Vec<u8> = row.get(5)?;
    let locked: bool = row.get(6)?;

    Ok((id, nonce, account_seed, code, storage, assets, locked))
}

/// Removes account states with the specified hashes from the database.
///
/// This is used to rollback account changes when a transaction is discarded,
/// effectively undoing the account state changes that were applied by the transaction.
///
/// Note: This is not part of the Store trait and is only used internally by the `SQLite` store
/// implementation to handle transaction rollbacks.
pub(crate) fn undo_account_state(
    tx: &Transaction<'_>,
    account_hashes: &[Digest],
) -> Result<(), StoreError> {
    const QUERY: &str = "DELETE FROM accounts WHERE account_commitment = ?";
    for account_id in account_hashes {
        tx.execute(QUERY, params![account_id.to_hex()])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use miden_objects::{
        account::{AccountCode, AccountComponent},
        testing::account_component::BASIC_WALLET_CODE,
    };

    use crate::store::sqlite_store::{account::insert_account_code, tests::create_test_store};

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
}

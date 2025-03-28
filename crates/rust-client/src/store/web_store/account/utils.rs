use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    Digest, Felt, Word,
    account::{Account, AccountCode, AccountHeader, AccountId, AccountStorage},
    asset::{Asset, AssetVault},
    utils::Deserializable,
};
use miden_tx::utils::Serializable;
use serde_wasm_bindgen::from_value;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

use super::{
    js_bindings::{
        idxdb_get_account_auth_by_pub_key, idxdb_insert_account_asset_vault,
        idxdb_insert_account_auth, idxdb_insert_account_code, idxdb_insert_account_record,
        idxdb_insert_account_storage,
    },
    models::{AccountAuthIdxdbObject, AccountRecordIdxdbObject},
};
use crate::store::{AccountStatus, StoreError};

pub async fn insert_account_code(account_code: &AccountCode) -> Result<(), JsValue> {
    let root = account_code.commitment().to_string();
    let code = account_code.to_bytes();

    let promise = idxdb_insert_account_code(root, code);
    JsFuture::from(promise).await?;

    Ok(())
}

pub async fn insert_account_storage(account_storage: &AccountStorage) -> Result<(), JsValue> {
    let root = account_storage.commitment().to_string();

    let storage = account_storage.to_bytes();

    let promise = idxdb_insert_account_storage(root, storage);
    JsFuture::from(promise).await?;

    Ok(())
}

pub async fn insert_account_asset_vault(asset_vault: &AssetVault) -> Result<(), JsValue> {
    let commitment = asset_vault.root().to_string();
    let assets = asset_vault.assets().collect::<Vec<Asset>>().to_bytes();

    let promise = idxdb_insert_account_asset_vault(commitment, assets);
    JsFuture::from(promise).await?;

    Ok(())
}

pub async fn insert_account_auth(pub_key: String, secret_key: String) -> Result<(), JsValue> {
    let promise = idxdb_insert_account_auth(pub_key, secret_key);
    JsFuture::from(promise).await?;

    Ok(())
}

pub fn get_account_auth_by_pub_key(pub_key: String) -> Result<String, StoreError> {
    let js_value = idxdb_get_account_auth_by_pub_key(pub_key.clone());
    let account_auth_idxdb: Option<AccountAuthIdxdbObject> = from_value(js_value)
        .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

    match account_auth_idxdb {
        Some(account_auth) => Ok(account_auth.secret_key),
        None => Err(StoreError::AccountKeyNotFound(pub_key)),
    }
}

pub async fn insert_account_record(
    account: &Account,
    account_seed: Option<Word>,
) -> Result<(), JsValue> {
    let account_id_str = account.id().to_string();
    let code_root = account.code().commitment().to_string();
    let storage_root = account.storage().commitment().to_string();
    let vault_root = account.vault().root().to_string();
    let committed = account.is_public();
    let nonce = account.nonce().to_string();
    let account_seed = account_seed.map(|seed| seed.to_bytes());
    let commitment = account.commitment().to_string();

    let promise = idxdb_insert_account_record(
        account_id_str,
        code_root,
        storage_root,
        vault_root,
        nonce,
        committed,
        account_seed,
        commitment,
    );
    JsFuture::from(promise).await?;

    Ok(())
}

pub fn parse_account_record_idxdb_object(
    account_header_idxdb: AccountRecordIdxdbObject,
) -> Result<(AccountHeader, AccountStatus), StoreError> {
    let native_account_id: AccountId = AccountId::from_hex(&account_header_idxdb.id)?;
    let native_nonce: u64 = account_header_idxdb
        .nonce
        .parse::<u64>()
        .map_err(|err| StoreError::ParsingError(err.to_string()))?;
    let account_seed = account_header_idxdb
        .account_seed
        .map(|seed| Word::read_from_bytes(&seed))
        .transpose()?;

    let account_header = AccountHeader::new(
        native_account_id,
        Felt::new(native_nonce),
        Digest::try_from(&account_header_idxdb.vault_root)?,
        Digest::try_from(&account_header_idxdb.storage_root)?,
        Digest::try_from(&account_header_idxdb.code_root)?,
    );

    let status = match (account_seed, account_header_idxdb.locked) {
        (_, true) => AccountStatus::Locked,
        (Some(seed), _) => AccountStatus::New { seed },
        _ => AccountStatus::Tracked,
    };

    Ok((account_header, status))
}

pub async fn update_account(new_account_state: &Account) -> Result<(), JsValue> {
    insert_account_storage(new_account_state.storage()).await?;
    insert_account_asset_vault(new_account_state.vault()).await?;
    insert_account_record(new_account_state, None).await
}

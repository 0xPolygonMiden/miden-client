use alloc::{string::ToString, vec::Vec};

use miden_objects::{
    account::{Account, AccountCode, AccountHeader, AccountId, AccountStorage},
    asset::{Asset, AssetVault},
    utils::Deserializable,
    Digest, Felt, Word,
};
use miden_tx::utils::Serializable;
use wasm_bindgen_futures::JsFuture;

use super::{
    js_bindings::{
        idxdb_insert_account_asset_vault, idxdb_insert_account_auth, idxdb_insert_account_code,
        idxdb_insert_account_record, idxdb_insert_account_storage,
    },
    models::AccountRecordIdxdbObject,
};
use crate::store::{AccountStatus, StoreError};

pub async fn insert_account_code(account_code: &AccountCode) -> Result<(), ()> {
    let root = account_code.commitment().to_string();
    let code = account_code.to_bytes();

    let promise = idxdb_insert_account_code(root, code);
    let _ = JsFuture::from(promise).await;

    Ok(())
}

pub async fn insert_account_storage(account_storage: &AccountStorage) -> Result<(), ()> {
    let root = account_storage.commitment().to_string();

    let storage = account_storage.to_bytes();

    let promise = idxdb_insert_account_storage(root, storage);
    let _ = JsFuture::from(promise).await;

    Ok(())
}

pub async fn insert_account_asset_vault(asset_vault: &AssetVault) -> Result<(), ()> {
    let commitment = asset_vault.commitment().to_string();
    let assets = asset_vault.assets().collect::<Vec<Asset>>().to_bytes();

    let promise = idxdb_insert_account_asset_vault(commitment, assets);
    let _ = JsFuture::from(promise).await;
    Ok(())
}

pub async fn insert_account_record(
    account: &Account,
    account_seed: Option<Word>,
) -> Result<(), ()> {
    let account_id_str = account.id().to_string();
    let code_root = account.code().commitment().to_string();
    let storage_root = account.storage().commitment().to_string();
    let vault_root = account.vault().commitment().to_string();
    let committed = account.is_public();
    let nonce = account.nonce().to_string();
    let account_seed = account_seed.map(|seed| seed.to_bytes());
    let hash = account.hash().to_string();

    let promise = idxdb_insert_account_record(
        account_id_str,
        code_root,
        storage_root,
        vault_root,
        nonce,
        committed,
        account_seed,
        hash,
    );
    let _ = JsFuture::from(promise).await;

    Ok(())
}

pub fn parse_account_record_idxdb_object(
    account_header_idxdb: AccountRecordIdxdbObject,
) -> Result<(AccountHeader, AccountStatus), StoreError> {
    let native_account_id: AccountId = AccountId::from_hex(&account_header_idxdb.id).unwrap();
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

pub async fn update_account(new_account_state: &Account) -> Result<(), ()> {
    insert_account_storage(new_account_state.storage()).await?;
    insert_account_asset_vault(new_account_state.vault()).await?;
    insert_account_record(new_account_state, None).await
}

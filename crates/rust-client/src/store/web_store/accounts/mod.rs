use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    accounts::{Account, AccountCode, AccountHeader, AccountId, AccountStorage, AuthSecretKey},
    assets::{Asset, AssetVault},
    Digest, Word,
};
use miden_tx::utils::{Deserializable, Serializable};
use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::*;

use super::WebStore;
use crate::store::StoreError;

mod js_bindings;
use js_bindings::*;

mod models;
use models::*;

pub(crate) mod utils;
use utils::*;

impl WebStore {
    pub(super) async fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        let promise = idxdb_get_account_ids();
        let js_value = JsFuture::from(promise).await.unwrap();
        let account_ids_as_strings: Vec<String> = from_value(js_value).unwrap();

        let native_account_ids: Vec<AccountId> = account_ids_as_strings
            .into_iter()
            .map(|id| AccountId::from_hex(&id).unwrap())
            .collect();

        Ok(native_account_ids)
    }

    pub(super) async fn get_account_headers(
        &self,
    ) -> Result<Vec<(AccountHeader, Option<Word>)>, StoreError> {
        let promise = idxdb_get_account_headers();
        let js_value = JsFuture::from(promise).await.unwrap();
        let account_headers_idxdb: Vec<AccountRecordIdxdbOjbect> = from_value(js_value).unwrap();

        let account_headers: Result<Vec<(AccountHeader, Option<Word>)>, StoreError> =
            account_headers_idxdb
                .into_iter()
                .map(parse_account_record_idxdb_object)
                .collect(); // Collect results into a single Result

        account_headers
    }

    pub(crate) async fn get_account_header(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountHeader, Option<Word>), StoreError> {
        let account_id_str = account_id.to_string();

        let promise = idxdb_get_account_header(account_id_str);
        let js_value = JsFuture::from(promise).await.unwrap();
        let account_header_idxdb: AccountRecordIdxdbOjbect = from_value(js_value).unwrap();

        parse_account_record_idxdb_object(account_header_idxdb)
    }

    pub(crate) async fn get_account_header_by_hash(
        &self,
        account_hash: Digest,
    ) -> Result<Option<AccountHeader>, StoreError> {
        let account_hash_str = account_hash.to_string();

        let promise = idxdb_get_account_header_by_hash(account_hash_str);
        let js_value = JsFuture::from(promise).await.unwrap();
        let account_header_idxdb: Option<AccountRecordIdxdbOjbect> = from_value(js_value).unwrap();

        let account_header: Result<Option<AccountHeader>, StoreError> = account_header_idxdb
            .map_or(Ok(None), |account_record| {
                let result = parse_account_record_idxdb_object(account_record);

                result.map(|(account_header, _account_seed)| Some(account_header))
            });

        account_header
    }

    pub(crate) async fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<(Account, Option<Word>), StoreError> {
        let (account_header, seed) = self.get_account_header(account_id).await.unwrap();
        let account_code = self.get_account_code(account_header.code_commitment()).await.unwrap();

        let account_storage =
            self.get_account_storage(account_header.storage_commitment()).await.unwrap();
        let account_vault = self.get_vault_assets(account_header.vault_root()).await.unwrap();
        let account_vault = AssetVault::new(&account_vault).unwrap();

        let account = Account::from_parts(
            account_header.id(),
            account_vault,
            account_storage,
            account_code,
            account_header.nonce(),
        );

        Ok((account, seed))
    }

    pub(super) async fn get_account_code(&self, root: Digest) -> Result<AccountCode, StoreError> {
        let root_serialized = root.to_string();

        let promise = idxdb_get_account_code(root_serialized);
        let js_value = JsFuture::from(promise).await.unwrap();
        let account_code_idxdb: AccountCodeIdxdbObject = from_value(js_value).unwrap();

        let code = AccountCode::from_bytes(&account_code_idxdb.code).unwrap();

        Ok(code)
    }

    pub(super) async fn get_account_storage(
        &self,
        commitment: Digest,
    ) -> Result<AccountStorage, StoreError> {
        let commitment_serialized = commitment.to_string();

        let promise = idxdb_get_account_storage(commitment_serialized);
        let js_value = JsFuture::from(promise).await.unwrap();
        let account_storage_idxdb: AccountStorageIdxdbObject = from_value(js_value).unwrap();

        let storage = AccountStorage::read_from_bytes(&account_storage_idxdb.storage).unwrap();
        Ok(storage)
    }

    pub(super) async fn get_vault_assets(
        &self,
        commitment: Digest,
    ) -> Result<Vec<Asset>, StoreError> {
        let commitment_serialized = serde_json::to_string(&commitment.to_string()).unwrap();

        let promise = idxdb_get_account_asset_vault(commitment_serialized);
        let js_value = JsFuture::from(promise).await.unwrap();
        let vault_assets_idxdb: AccountVaultIdxdbObject = from_value(js_value).unwrap();

        let assets = Vec::<Asset>::read_from_bytes(&vault_assets_idxdb.assets).unwrap();
        Ok(assets)
    }

    pub(crate) async fn get_account_auth(
        &self,
        account_id: AccountId,
    ) -> Result<AuthSecretKey, StoreError> {
        let account_id_str = account_id.to_string();

        let promise = idxdb_get_account_auth(account_id_str);
        let js_value = JsFuture::from(promise).await.unwrap();
        let auth_info_idxdb: AccountAuthIdxdbObject = from_value(js_value).unwrap();

        // Convert the auth_info to the appropriate AuthInfo enum variant
        let auth_info = AuthSecretKey::read_from_bytes(&auth_info_idxdb.auth_info)?;

        Ok(auth_info)
    }

    pub(crate) async fn insert_account(
        &self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthSecretKey,
    ) -> Result<(), StoreError> {
        insert_account_code(account.code()).await.unwrap();

        insert_account_storage(account.storage()).await.unwrap();

        insert_account_asset_vault(account.vault()).await.unwrap();

        insert_account_record(account, account_seed).await.unwrap();

        insert_account_auth(account.id(), auth_info).await.unwrap();

        Ok(())
    }

    /// Returns an [AuthSecretKey] by a public key represented by a [Word]
    pub fn get_account_auth_by_pub_key(&self, pub_key: Word) -> Result<AuthSecretKey, StoreError> {
        let pub_key_bytes = pub_key.to_bytes();

        let js_value = idxdb_get_account_auth_by_pub_key(pub_key_bytes);
        let account_auth_idxdb: AccountAuthIdxdbObject = from_value(js_value).unwrap();

        // Convert the auth_info to the appropriate AuthInfo enum variant
        let auth_info = AuthSecretKey::read_from_bytes(&account_auth_idxdb.auth_info)?;

        Ok(auth_info)
    }

    /// Fetches an [AuthSecretKey] by a public key represented by a [Word] and caches it in the
    /// store. This is used in the web_client so adding this to ignore the dead code warning.
    pub async fn fetch_and_cache_account_auth_by_pub_key(
        &self,
        account_id: String,
    ) -> Result<AuthSecretKey, StoreError> {
        let promise = idxdb_fetch_and_cache_account_auth_by_pub_key(account_id);
        let js_value = JsFuture::from(promise).await.unwrap();
        let account_auth_idxdb: AccountAuthIdxdbObject = from_value(js_value).unwrap();

        // Convert the auth_info to the appropriate AuthInfo enum variant
        let auth_info = AuthSecretKey::read_from_bytes(&account_auth_idxdb.auth_info)?;

        Ok(auth_info)
    }
}

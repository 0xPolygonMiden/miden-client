use miden_lib::transaction::TransactionKernel;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::*;
use web_sys::console;
use wasm_bindgen::prelude::*;

use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountStub}, assembly::ModuleAst, assets::{Asset, AssetVault}, Digest, Felt, Word
};
use miden_objects::utils::Deserializable;

use crate::native_code::{errors::StoreError, store::{AuthInfo, NoteFilter, Store}}; 

use super::WebStore;

mod js_bindings;
use js_bindings::*;

mod models;
use models::*;

pub(crate) mod utils;
use utils::*;

impl WebStore {
    pub(super) async fn get_account_ids(
        &self
    ) -> Result<Vec<AccountId>, StoreError> {
        let promise = idxdb_get_account_ids();
        let js_value = JsFuture::from(promise).await.unwrap();
        let account_ids_as_strings: Vec<String> = from_value(js_value).unwrap();
  
        let native_account_ids: Vec<AccountId> = account_ids_as_strings.into_iter().map(|id| {
            AccountId::from_hex(&id).unwrap()
        }).collect();
        
        return Ok(native_account_ids);
    }

    pub(super) async fn get_account_stubs(
        &self
    ) -> Result<Vec<(AccountStub, Option<Word>)>, StoreError> {
        let promise = idxdb_get_account_stubs();
        let js_value = JsFuture::from(promise).await.unwrap();
        let account_stubs_idxdb: Vec<AccountRecordIdxdbOjbect> = from_value(js_value).unwrap();
        
        let account_stubs: Vec<(AccountStub, Option<Word>)> = account_stubs_idxdb.into_iter().map(|record| {
            let native_account_id: AccountId = AccountId::from_hex(&record.id).unwrap();
            let native_nonce: u64 = record.nonce.parse::<u64>().unwrap();
            let account_seed = record.account_seed.map(|seed| Word::read_from_bytes(&seed)).transpose().unwrap();
            
            let account_stub = AccountStub::new(
                native_account_id,
                Felt::new(native_nonce),
                Digest::try_from(&record.vault_root).unwrap(),
                Digest::try_from(&record.storage_root).unwrap(),
                Digest::try_from(&record.code_root).unwrap(),
            );

            (account_stub, account_seed) // Adjust this as needed based on how you derive Word from your data
        }).collect();

        Ok(account_stubs)
    }

    pub(crate) async fn get_account_stub(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), StoreError> {
        let account_id_str = account_id.to_string();
        
        let promise = idxdb_get_account_stub(account_id_str);
        let js_value = JsFuture::from(promise).await.unwrap();
        let account_stub_idxdb: AccountRecordIdxdbOjbect = from_value(js_value).unwrap();

        let native_account_id: AccountId = AccountId::from_hex(&account_stub_idxdb.id).unwrap();
        let native_nonce: u64 = account_stub_idxdb.nonce.parse::<u64>().unwrap();
        let account_seed = account_stub_idxdb.account_seed.map(|seed| Word::read_from_bytes(&seed)).transpose().unwrap();

        Ok((
            AccountStub::new(
                native_account_id,
                Felt::new(native_nonce),
                Digest::try_from(&account_stub_idxdb.vault_root).unwrap(),
                Digest::try_from(&account_stub_idxdb.storage_root).unwrap(),
                Digest::try_from(&account_stub_idxdb.code_root).unwrap(),
            ),
            account_seed,
        ))
    }

    pub(crate) async fn get_account(
        &self,
        account_id: AccountId
    ) -> Result<(Account, Option<Word>), StoreError> {
        let (account_stub, seed) = self.get_account_stub(account_id).await.unwrap();

        let (_procedures, module_ast) = self.get_account_code(account_stub.code_root()).await.unwrap();

        let account_code = AccountCode::new(module_ast, &TransactionKernel::assembler()).unwrap();

        let account_storage = self.get_account_storage(account_stub.storage_root()).await.unwrap();

        let account_vault = self.get_vault_assets(account_stub.vault_root()).await.unwrap();
        let account_vault = AssetVault::new(&account_vault).unwrap();

        let account = Account::new(
            account_stub.id(),
            account_vault,
            account_storage,
            account_code,
            account_stub.nonce(),
        );

        Ok((account, seed))
    }

    pub(super) async fn get_account_code(
        &self,
        root: Digest
    ) -> Result<(Vec<Digest>, ModuleAst), StoreError> {
        let root_serialized = root.to_string();

        let promise = idxdb_get_account_code(root_serialized);
        let js_value = JsFuture::from(promise).await.unwrap();
        let account_code_idxdb: AccountCodeIdxdbObject = from_value(js_value).unwrap();

        let procedures =
            serde_json::from_str(&account_code_idxdb.procedures).unwrap();

        let module = ModuleAst::from_bytes(&account_code_idxdb.module).unwrap();
        
        Ok((procedures, module))
    }
    
    pub(super) async fn get_account_storage(
        &self,
        root: Digest
    ) -> Result<AccountStorage, StoreError> {
        let root_serialized = root.to_string();

        let promise = idxdb_get_account_storage(root_serialized);
        let js_value = JsFuture::from(promise).await.unwrap();
        let account_storage_idxdb: AccountStorageIdxdbObject = from_value(js_value).unwrap();

        let storage = AccountStorage::read_from_bytes(&account_storage_idxdb.storage).unwrap();
        Ok(storage)
    }

    pub(super) async fn get_vault_assets(
        &self,
        root: Digest
    ) -> Result<Vec<Asset>, StoreError> {
        let root_serialized = root.to_string();

        let promise = idxdb_get_account_asset_vault(root_serialized);
        let js_value = JsFuture::from(promise).await.unwrap();
        let vault_assets_idxdb: AccountVaultIdxdbObject = from_value(js_value).unwrap();

        let assets = serde_json::from_str(&vault_assets_idxdb.assets).unwrap();
        Ok(assets)
    }

    pub(crate) async fn get_account_auth(
        &self,
        account_id: AccountId
    ) -> Result<AuthInfo, StoreError> {
        let account_id_str = account_id.to_string();

        let promise = idxdb_get_account_auth(account_id_str);
        let js_value = JsFuture::from(promise).await.unwrap();
        let auth_info_idxdb: AccountAuthIdxdbObject = from_value(js_value).unwrap();
        
        // Convert the auth_info to the appropriate AuthInfo enum variant
        let auth_info = AuthInfo::read_from_bytes(&auth_info_idxdb.auth_info).unwrap();

        Ok(auth_info)
    }

    pub(crate) async fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthInfo,
    ) -> Result<(), StoreError> {
        insert_account_code(account.code()).await;
        insert_account_storage(account.storage()).await;
        insert_account_asset_vault(account.vault()).await;
        insert_account_record(account, account_seed).await;
        insert_account_auth(account.id(), auth_info).await;

        Ok(())
    }
}
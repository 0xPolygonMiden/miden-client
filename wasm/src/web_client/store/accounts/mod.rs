// use wasm_bindgen_futures::*;
// use serde_wasm_bindgen::from_value;

use super::WebStore;

mod js_bindings;
// use js_bindings::*;

mod models;
// use models::*;

mod utils;
// use utils::*;

impl WebStore {
    // pub(super) async fn get_account_ids(
    //     &mut self
    // ) -> Result<Vec<AccountId>, ()> {
    //     let promise = idxdb_get_account_ids();
    //     let js_value = JsFuture::from(promise).await?;
    //     let account_ids_as_strings: Vec<String> = from_value(js_value).unwrap();
  
    //     let native_account_ids: Result<Vec<AccountId>, ()> = account_ids_as_strings.into_iter().map(|id| {
    //         AccountId::from_hex(&id).map_err(|_err| ()) // Convert any error to `()`
    //     }).collect(); // Collect into a Result<Vec<AccountId>, ()>
        
    //     return native_account_ids;
    // }

    // pub(super) async fn get_account_stubs(
    //     &mut self
    // ) ->  Result<Vec<(AccountStub, Option<Word>)>, ()> {
    //     let promise = idxdb_get_account_stubs();
    //     let js_value = JsFuture::from(promise).await?;
    //     let account_stubs_idxdb: Vec<AccountRecordIdxdbOjbect> = from_value(js_value).unwrap();
        
    //     let account_stubs: Vec<(AccountStub, Option<Word>)> = account_stubs_idxdb.into_iter().map(|record| {
    //         // Need to convert the hex string back to AccountId to then turn it into a u64
    //         let native_account_id: i64 = AccountId::from_hex(&record.id).map_err(|err| err.to_string())?;
    //         let native_nonce: i64 = record.nonce.parse().unwrap();
    //         let account_seed = record.account_seed.map(|seed| Word::read_from_bytes(&seed)).transpose()?;
            
    //         let account_stub = AccountStub {
    //             id: native_account_id,
    //             nonce: native_nonce,
    //             vault_root: record.vault_root,
    //             storage_root: record.storage_root,
    //             code_root: record.code_root,
    //             account_seed: account_seed,
    //         };

    //         (account_stub, None) // Adjust this as needed based on how you derive Word from your data
    //     }).collect();

    //     Ok(account_stubs)
    // }

    // pub(crate) async fn get_account_stub(
    //     &self,
    //     account_id: AccountId,
    // ) -> Result<(AccountStub, Option<Word>), ()> {
    //     let account_id_str = AccountId::to_hex(account_id).map_err(|err| err.to_string())?;
        
    //     let promise = idxdb_get_account_stub(account_id_str);
    //     let js_value = JsFuture::from(promise).await?;
    //     let account_stub_idxdb: AccountRecordIdxdbOjbect = from_value(js_value).unwrap();

    //     let native_account_id: i64 = AccountId::from_hex(&account_stub_idxdb.id).map_err(|err| err.to_string())?;
    //     let native_nonce: i64 = account_stub_idxdb.nonce.parse().unwrap();
    //     let account_seed = account_stub_idxdb.account_seed.map(|seed| Word::read_from_bytes(&seed)).transpose()?;

    //     Ok((
    //         AccountStub::new(
    //             (native_account_id as u64)
    //                 .try_into()
    //                 .expect("Conversion from stored AccountID should not panic"),
    //             Felt::new(native_nonce as u64),
    //             serde_json::from_str(&account_stub_idxdb.vault_root).map_err(|err| err.to_string())?,
    //             Digest::try_from(&account_stub_idxdb.storage_root)?,
    //             Digest::try_from(&account_stub_idxdb.code_root)?,
    //         ),
    //         account_seed,
    //     ));
    // }

    // pub(crate) async fn get_account(
    //     &mut self,
    //     account_id: AccountId
    // ) -> Result<(), ()> { // TODO: Replace with  Result<(Account, Option<Word>), ()>
    //     let (account_stub, seed) = self.get_account_stub(account_id)?;
    //     let (_procedures, module_ast) = self.get_account_code(account_stub.code_root())?;

    //     let account_code = AccountCode::new(module_ast, &TransactionKernel::assembler()).unwrap();

    //     let account_storage = self.get_account_storage(account_stub.storage_root())?;

    //     let account_vault = self.get_vault_assets(account_stub.vault_root())?;
    //     let account_vault = AssetVault::new(&account_vault)?;

    //     let account = Account::new(
    //         account_stub.id(),
    //         account_vault,
    //         account_storage,
    //         account_code,
    //         account_stub.nonce(),
    //     );

    //     Ok((account, seed))
    // }

    // pub(crate) async fn get_account_auth(
    //     &mut self,
    //     account_id: AccountId
    // ) -> Result<AuthInfo, ()> {
    //     let account_id_str = AccountId::to_hex(account_id).map_err(|err| err.to_string())?;

    //     let promise = idxdb_get_account_auth(account_id_str);
    //     let js_value = JsFuture::from(promise).await?;
    //     let auth_info_idxdb: AccountAuthIdxdbObject = from_value(js_value).unwrap();
        
    //     // Convert the auth_info to the appropriate AuthInfo enum variant
    //     let auth_info = AuthInfo::from_bytes(&auth_info_idxdb.auth_info);
    //     Ok(auth_info)
    // }

    // pub(super) async fn get_account_code(
    //     &mut self,
    //     root: Digest
    // ) -> Result<(Vec<Digest>, ModuleAst), ()> {
    //     let root_serialized = root.to_string();

    //     let promise = idxdb_get_account_code(root_serialized);
    //     let js_value = JsFuture::from(promise).await?;
    //     let account_code_idxdb: AccountCodeIdxdbObject = from_value(js_value).unwrap();

    //     let procedures =
    //         serde_json::from_str(&account_code_idxdb.procedures)?;
    //     let module = ModuleAst::from_bytes(&account_code_idxdb.module)?;
    //     Ok((procedures, module));
    // }
    
    // pub(super) async fn get_account_storage(
    //     &mut self,
    //     root: Digest
    // ) -> Result<AccountStorage, ()> {
    //     let root_serialized = &root.to_string();

    //     let promise = idxdb_get_account_storage(root_serialized);
    //     let js_value = JsFuture::from(promise).await?;
    //     let account_storage_idxdb: AccountStorageIdxdbObject = from_value(js_value).unwrap();

    //     let storage = AccountStorage::from_bytes(&account_code_idxdb.storage);
    //     Ok(storage)
    // }

    // pub(super) async fn get_vault_assets(
    //     &mut self,
    //     root: Digest
    // ) -> Result<Vec<Asset>, ()> {
    //     let root_serialized = &root.to_string();

    //     let promise = idxdb_get_account_asset_vault(root_serialized);
    //     let js_value = JsFuture::from(promise).await?;
    //     let vault_assets_idxdb: AccountVaultIdxdbObject = from_value(js_value).unwrap();

    //     let assets = serde_json::from_str(&vault_assets_idxdb.assets);
    //     Ok(assets)
    // }

    // pub(crate) async fn insert_account(
    //     &mut self,
    //     account: &Account,
    //     account_seed: Option<Word>,
    //     auth_info: &AuthInfo,
    // ) -> Result<(), ()> {
    //     insert_account_code(account.code()).await?;
    //     insert_account_storage(account.storage()).await?;
    //     insert_account_asset_vault(account.vault()).await?;
    //     insert_account_record(account, account_seed).await?;
    //     insert_account_auth(account.id(), auth_info).await?;

    //     Ok(())
    // }
}
use crate::native_code::store::Store; 

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

use async_trait::async_trait;

// Example
#[wasm_bindgen(module = "/js/db/greet.js")]
extern "C" {
    #[wasm_bindgen(js_name = insertGreeting)]
    fn insert_greeting(greeting: String) -> js_sys::Promise;
}

// Initialize IndexedDB
#[wasm_bindgen(module = "/js/db/schema.js")]
extern "C" {
    #[wasm_bindgen(js_name = openDatabase)]
    fn setup_indexed_db() -> js_sys::Promise;
}

// Account IndexedDB Operations
#[wasm_bindgen(module = "/js/db/accounts.js")]
extern "C" {
    #[wasm_bindgen(js_name = insertAccountCode)]
    fn idxdb_insert_account_code(
        code_root: String, 
        code: String, 
        module: Vec<u8>
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertAccountStorage)]
    fn idxdb_insert_account_storage(
        storage_root: String, 
        storage_slots: Vec<u8>
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertAccountAssetVault)]
    fn idxdb_insert_account_asset_vault(
        vault_root: String, 
        assets: String
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertAccountAuth)]
    fn idxdb_insert_account_auth(
        id: i64, 
        auth_info: Vec<u8>
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertAccountRecord)]
    fn idxdb_insert_account_record(
        id: i64, 
        code_root: String, 
        storage_root: String, 
        vault_root: String, 
        nonce: i64, 
        committed: bool, 
        account_seed: Option<Vec<u8>>
    ) -> js_sys::Promise;
}

// TYPES

type SerializedAccountCodeData = (String, String, Vec<u8>);
type SerializedAccountStorageData = (String, Vec<u8>);
type SerializedAccountVaultData = (String, String);
type SerializedAccountAuthData = (i64, Vec<u8>);
type SerializedAccountData = (i64, String, String, String, i64, bool);

// ================================================================================================

pub struct WebStore {}

impl WebStore {
    pub async fn new() -> Result<WebStore, ()> {
        let _ = JsFuture::from(setup_indexed_db()).await;
        Ok(WebStore {})
    }
}

#[async_trait(?Send)]
impl Store for WebStore {
    async fn insert_string(
        &mut self, 
        data: String
    ) -> Result<(), ()> {
        let result = JsFuture::from(insert_greeting(data)).await;
        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    // async fn insert_account(
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

// async fn insert_account_code(
//     account_code: &AccountCode
// ) -> Result<(), ()> {
//     let (code_root, code, module) = serialize_account_code(account_code)?;
//     let result = JsFuture::from(idxdb_insert_account_code(code_root, code, module)).await;
//     match result {
//         Ok(_) => Ok(()),
//         Err(_) => Err(()),
//     }
// }

// fn serialize_account_code(
//     account_code: &AccountCode,
// ) -> Result<SerializedAccountCodeData, ()> {
//     let root = account_code.root().to_string();
//     let procedures = match serde_json::to_string(account_code.procedures()) {
//         Ok(procedures) => procedures,
//         Err(_) => return Err(()),
//     };
//     // Assuming to_bytes() returns a Result and handling its error similarly
//     let module = match account_code.module().to_bytes(AstSerdeOptions {
//         serialize_imports: true,
//     }) {
//         Ok(module) => module,
//         Err(_) => return Err(()),
//     };

//     Ok((root, procedures, module))
// }

// async fn insert_account_storage(
//     account_storage: &AccountStorage
// ) -> Result<(), ()> {
//     let (storage_root, storage_slots) = serialize_account_storage(account_storage)?;
//     let result = JsFuture::from(idxdb_insert_account_storage(storage_root, storage_slots)).await;
//     match result {
//         Ok(_) => Ok(()),
//         Err(_) => Err(()),
//     }
// }

// fn serialize_account_storage(
//     account_storage: &AccountStorage,
// ) -> Result<SerializedAccountStorageData, ()> {
//     let root = account_storage.root().to_string();
//     let storage = account_storage.to_bytes();

//     Ok((root, storage))
// }

// async fn insert_account_asset_vault(
//     asset_vault: &AssetVault
// ) -> Result<(), ()> {
//     let (vault_root, assets) = serialize_account_asset_vault(asset_vault)?;
//     let result = JsFuture::from(idxdb_ insert_account_asset_vault(vault_root, assets)).await;
//         match result {
//             Ok(_) => Ok(()),
//             Err(_) => Err(()),
//         }
// }

// fn serialize_account_asset_vault(
//     asset_vault: &AssetVault,
// ) -> Result<SerializedAccountVaultData, ()> {
//     let root = match serde_json::to_string(&asset_vault.commitment()) {
//         Ok(root) => root,
//         Err(_) => return Err(()),
//     };
//     let assets: Vec<Asset> = asset_vault.assets().collect();
//     let assets = match serde_json::to_string(&assets) {
//         Ok(assets) => assets,
//         Err(_) => return Err(()),
//     };
//     Ok((root, assets))
// }

// async fn insert_account_record(
//     account: &Account,
//     account_seed: Option<Word>,
// ) -> Result<(), ()> {
//     let (id, code_root, storage_root, vault_root, nonce, committed) = serialize_account(account)?;
//     let account_seed = account_seed.map(|seed| seed.to_bytes());

//     let result = JsFuture::from(idxdb_insert_account_record(
//         id,
//         code_root,
//         storage_root,
//         vault_root,
//         nonce,
//         committed,
//         account_seed,
//     )).await;
//     match result {
//         Ok(_) => Ok(()),
//         Err(_) => Err(()),
//     }
// }

// fn serialize_account(account: &Account) -> Result<SerializedAccountData, ()> {
//     let id: u64 = account.id().into();
//     let code_root = account.code().root().to_string();
//     let storage_root = account.storage().root().to_string();
//     let vault_root = match serde_json::to_string(&account.vault().commitment()) {
//         Ok(vault_root) => vault_root,
//         Err(_) => return Err(()),
//     };
//     let committed = account.is_on_chain();
//     let nonce = account.nonce().as_int() as i64;

//     Ok((
//         id as i64,
//         code_root,
//         storage_root,
//         vault_root,
//         nonce,
//         committed,
//     ))
// }

// async fn insert_account_auth(
//     account_id: AccountId,
//     auth_info: &AuthInfo,
// ) -> Result<(), ()> {
//     let (account_id, auth_info) = serialize_account_auth(account_id, auth_info)?;
//     let result = JsFuture::from(idxdb_insert_account_auth(account_id, auth_info)).await;
//     match result {
//         Ok(_) => Ok(()),
//         Err(_) => Err(()),
//     }
// }

// fn serialize_account_auth(
//     account_id: AccountId,
//     auth_info: &AuthInfo,
// ) -> Result<SerializedAccountAuthData, ()> {
//     let account_id: u64 = account_id.into();
//     let auth_info = auth_info.to_bytes();
//     Ok((account_id as i64, auth_info))
// }
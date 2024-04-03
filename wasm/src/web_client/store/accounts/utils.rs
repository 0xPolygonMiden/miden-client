// use wasm_bindgen_futures::*;

// use super::js_bindings::*;

// TYPES
// ================================================================================================

// type SerializedAccountCodeData = (String, String, Vec<u8>);
// type SerializedAccountStorageData = (String, Vec<u8>);
// type SerializedAccountVaultData = (String, String);
// type SerializedAccountAuthData = (String, Vec<u8>);
// type SerializedAccountData = (String, String, String, String, String, bool);

// ================================================================================================

// pub async fn insert_account_code(
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

// pub async fn insert_account_storage(
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

// pub async fn insert_account_asset_vault(
//     asset_vault: &AssetVault
// ) -> Result<(), ()> {
//     let (vault_root, assets) = serialize_account_asset_vault(asset_vault)?;
//     let result = JsFuture::from(idxdb_insert_account_asset_vault(vault_root, assets)).await;
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

// pub async fn insert_account_record(
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
//     let account_id_str = AccountId::to_hex(account.id()).map_err(|err| err.to_string())?;
//     let code_root = account.code().root().to_string();
//     let storage_root = account.storage().root().to_string();
//     let vault_root = match serde_json::to_string(&account.vault().commitment()) {
//         Ok(vault_root) => vault_root,
//         Err(_) => return Err(()),
//     };
//     let committed = account.is_on_chain();
//     let nonce = account.nonce().to_string();

//     Ok((
//         account_id_str,
//         code_root,
//         storage_root,
//         vault_root,
//         nonce,
//         committed,
//     ))
// }

// pub async fn insert_account_auth(
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
//     let account_id_str = AccountId::to_hex(account_id).map_err(|err| err.to_string())?;
//     let auth_info = auth_info.to_bytes();
//     Ok((account_id_str, auth_info))
// }
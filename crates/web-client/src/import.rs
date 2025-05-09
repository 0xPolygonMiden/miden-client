use miden_client::auth::AuthSecretKey;
use miden_objects::{
    account::{AccountFile, AccountId as NativeAccountId},
    note::NoteFile,
    utils::Deserializable,
};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::*;

use crate::{
    WebClient,
    helpers::generate_wallet,
    js_error_with_context,
    models::{
        account::Account, account_id::AccountId as JsAccountId,
        account_storage_mode::AccountStorageMode,
    },
};

#[wasm_bindgen]
impl WebClient {
    #[wasm_bindgen(js_name = "importAccount")]
    pub async fn import_account(&mut self, account_bytes: JsValue) -> Result<JsValue, JsValue> {
        let keystore = self.keystore.clone();
        if let Some(client) = self.get_mut_inner() {
            let account_bytes_result: Vec<u8> =
                from_value(account_bytes).map_err(|err| err.to_string())?;
            let account_data = AccountFile::read_from_bytes(&account_bytes_result)
                .map_err(|err| err.to_string())?;
            let account_id = account_data.account.id().to_string();

            client
                .add_account(&account_data.account, account_data.account_seed, false)
                .await
                .map_err(|err| js_error_with_context(err, "failed to import account"))?;

            keystore
                .expect("KeyStore should be initialized")
                .add_key(&account_data.auth_secret_key)
                .await
                .map_err(|err| err.to_string())?;

            Ok(JsValue::from_str(&format!("Imported account with ID: {account_id}")))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    #[wasm_bindgen(js_name = "importPublicAccountFromSeed")]
    pub async fn import_public_account_from_seed(
        &mut self,
        init_seed: Vec<u8>,
        mutable: bool,
    ) -> Result<Account, JsValue> {
        let keystore = self.keystore.clone();
        let client = self.get_mut_inner().ok_or(JsValue::from_str("Client not initialized"))?;

        let (generated_acct, _, key_pair) =
            generate_wallet(client, &AccountStorageMode::public(), mutable, Some(init_seed))
                .await?;

        let native_id = generated_acct.id();
        client
            .import_account_by_id(native_id)
            .await
            .map_err(|err| js_error_with_context(err, "failed to import public account"))?;

        keystore
            .expect("KeyStore should be initialized")
            .add_key(&AuthSecretKey::RpoFalcon512(key_pair))
            .await
            .map_err(|err| err.to_string())?;

        Ok(Account::from(generated_acct))
    }

    #[wasm_bindgen(js_name = "importAccountById")]
    pub async fn import_account_by_id(
        &mut self,
        account_id: &JsAccountId,
    ) -> Result<JsValue, JsValue> {
        let client = self
            .get_mut_inner()
            .ok_or_else(|| JsValue::from_str("Client not initialized"))?;

        let native_id: NativeAccountId = account_id.into();

        client
            .import_account_by_id(native_id)
            .await
            .map(|_| JsValue::undefined())
            .map_err(|err| js_error_with_context(err, "failed to import public account"))
    }

    #[wasm_bindgen(js_name = "importNote")]
    pub async fn import_note(&mut self, note_bytes: JsValue) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_bytes_result: Vec<u8> =
                from_value(note_bytes).map_err(|err| err.to_string())?;

            let note_file =
                NoteFile::read_from_bytes(&note_bytes_result).map_err(|err| err.to_string())?;

            let imported = client
                .import_note(note_file)
                .await
                .map_err(|err| js_error_with_context(err, "failed to import note"))?;

            Ok(JsValue::from_str(&imported.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    #[wasm_bindgen(js_name = "forceImportStore")]
    pub async fn force_import_store(&mut self, store_dump: JsValue) -> Result<JsValue, JsValue> {
        let store = self.store.as_ref().ok_or(JsValue::from_str("Store not initialized"))?;
        store
            .force_import_store(store_dump)
            .await
            .map_err(|err| js_error_with_context(err, "failed to force import store"))?;

        Ok(JsValue::from_str("Store imported successfully"))
    }
}

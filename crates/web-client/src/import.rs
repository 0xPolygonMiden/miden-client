use miden_client::auth::AuthSecretKey;
use miden_objects::{
    account::{AccountFile, AccountId},
    note::NoteFile,
    utils::Deserializable,
};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::*;

use super::models::account::Account;
use crate::{
    WebClient, helpers::generate_wallet, js_error_with_context,
    models::account_storage_mode::AccountStorageMode,
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

            keystore
                .expect("KeyStore should be initialized")
                .add_key(&account_data.auth_secret_key)
                .await
                .map_err(|err| err.to_string())?;
            match client
                .add_account(&account_data.account, account_data.account_seed, false)
                .await
            {
                Ok(_) => {
                    let message = format!("Imported account with ID: {account_id}");
                    Ok(JsValue::from_str(&message))
                },
                Err(err) => Err(js_error_with_context(err, "failed to import account")),
            }
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

        keystore
            .expect("KeyStore should be initialized")
            .add_key(&AuthSecretKey::RpoFalcon512(key_pair))
            .await
            .map_err(|err| err.to_string())?;

        let account_id = generated_acct.id();
        client
            .import_account_by_id(account_id)
            .await
            .map_err(|err| js_error_with_context(err, "failed to import account"))?;

        Ok(Account::from(generated_acct))
    }

    #[wasm_bindgen(js_name = "importAccountById")]
    pub async fn import_account_by_id(&mut self, account_id: String) -> Result<Account, JsValue> {
        let client = self
            .get_mut_inner()
            .ok_or_else(|| JsValue::from_str("Client not initialized"))?;

        let account_id = AccountId::from_hex(&account_id)
            .map_err(|e| JsValue::from_str(&format!("invalid account id: {}", e)))?;

        client
            .import_account_by_id(account_id.clone())
            .await
            .map_err(|err| js_error_with_context(err, "failed to import public account"))?;

        let record_opt = client
            .get_account(account_id)
            .await
            .map_err(|err| js_error_with_context(err, "failed to retrieve account"))?;

        if let Some(record) = record_opt {
            let inner_acct = record.account().clone();
            Ok(Account::from(inner_acct))
        } else {
            Err(JsValue::from_str("failed to parse account record"))
        }
    }

    #[wasm_bindgen(js_name = "importNote")]
    pub async fn import_note(&mut self, note_bytes: JsValue) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_bytes_result: Vec<u8> =
                from_value(note_bytes).map_err(|err| err.to_string())?;

            let note_file =
                NoteFile::read_from_bytes(&note_bytes_result).map_err(|err| err.to_string())?;

            Ok(client
                .import_note(note_file)
                .await
                .map_err(|err| js_error_with_context(err, "failed to import note"))?
                .to_string()
                .into())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    // Destructive operation, will fully overwrite the current web store
    //
    // The input to this function should be the result of a call to `export_store`
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

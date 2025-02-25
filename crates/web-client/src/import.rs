use miden_client::authenticator::keystore::KeyStore;
use miden_objects::{account::AccountFile, note::NoteFile, utils::Deserializable};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::*;

use super::models::account::Account;
use crate::{
    helpers::generate_account, models::account_storage_mode::AccountStorageMode, WebClient,
};

#[wasm_bindgen]
impl WebClient {
    pub async fn import_account(&mut self, account_bytes: JsValue) -> Result<JsValue, JsValue> {
        let keystore = self.keystore.clone();
        if let Some(client) = self.get_mut_inner() {
            let account_bytes_result: Vec<u8> = from_value(account_bytes).unwrap();
            let account_data = AccountFile::read_from_bytes(&account_bytes_result)
                .map_err(|err| err.to_string())?;
            let account_id = account_data.account.id().to_string();

            keystore
                .expect("KeyStore should be initialized")
                .add_key(&account_data.auth_secret_key)
                .map_err(|err| err.to_string())?;
            match client
                .add_account(&account_data.account, account_data.account_seed, false)
                .await
            {
                Ok(_) => {
                    let message = format!("Imported account with ID: {account_id}");
                    Ok(JsValue::from_str(&message))
                },
                Err(err) => {
                    let error_message = format!("Failed to import account: {err:?}");
                    Err(JsValue::from_str(&error_message))
                },
            }
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn import_public_account_from_seed(
        &mut self,
        init_seed: Vec<u8>,
        mutable: bool,
    ) -> Result<Account, JsValue> {
        let client = self.get_mut_inner().ok_or(JsValue::from_str("Client not initialized"))?;

        let (generated_acct, ..) =
            generate_account(client, &AccountStorageMode::public(), mutable, Some(init_seed))
                .await?;

        let account_id = generated_acct.id();
        client.import_account_by_id(account_id).await.map_err(|err| {
            let error_message = format!("Failed to import account: {err:?}");
            JsValue::from_str(&error_message)
        })?;

        Ok(Account::from(generated_acct))
    }
    pub async fn import_note(&mut self, note_bytes: JsValue) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_bytes_result: Vec<u8> = from_value(note_bytes).unwrap();

            let note_file =
                NoteFile::read_from_bytes(&note_bytes_result).map_err(|err| err.to_string())?;

            match client.import_note(note_file).await {
                Ok(note_id) => Ok(JsValue::from_str(note_id.to_string().as_str())),
                Err(err) => {
                    let error_message = format!("Failed to import note: {err:?}");
                    Err(JsValue::from_str(&error_message))
                },
            }
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

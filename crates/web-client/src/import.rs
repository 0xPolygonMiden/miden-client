use miden_client::auth::AuthSecretKey;
use miden_objects::{account::AccountData, note::NoteFile, utils::Deserializable};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::*;

use super::models::account::Account;
use crate::{
    helpers::generate_account, models::account_storage_mode::AccountStorageMode, WebClient,
};

#[wasm_bindgen]
impl WebClient {
    pub async fn import_account(&mut self, account_bytes: JsValue) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let account_bytes_result: Vec<u8> = from_value(account_bytes).unwrap();
            let account_data = AccountData::read_from_bytes(&account_bytes_result)
                .map_err(|err| err.to_string())?;
            let account_id = account_data.account.id().to_string();

            match client
                .add_account(
                    &account_data.account,
                    account_data.account_seed,
                    &account_data.auth_secret_key,
                    false,
                )
                .await
            {
                Ok(_) => {
                    let message = format!("Imported account with ID: {}", account_id);
                    Ok(JsValue::from_str(&message))
                },
                Err(err) => {
                    let error_message = format!("Failed to import account: {:?}", err);
                    Err(JsValue::from_str(&error_message))
                },
            }
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn import_account_from_seed(
        &mut self,
        init_seed: Vec<u8>,
        storage_mode: &AccountStorageMode,
        mutable: bool,
    ) -> Result<Account, JsValue> {
        let client = self.get_mut_inner().ok_or(JsValue::from_str("Client not initialized"))?;

        let (generated_acct, account_seed, key_pair) =
            generate_account(client, storage_mode, mutable, Some(init_seed)).await?;

        if storage_mode.is_public() {
            // If public, fetch the data from chain
            let account_details =
                client.get_account_details(generated_acct.id()).await.map_err(|err| {
                    JsValue::from_str(&format!("Failed to get account details: {}", err))
                })?;

            let on_chain_account = account_details
                .account()
                .ok_or(JsValue::from_str("Account not found on chain"))?;

            client
                .add_account(on_chain_account, None, &AuthSecretKey::RpoFalcon512(key_pair), false)
                .await
                .map_err(|err| JsValue::from_str(&format!("Failed to import account: {:?}", err)))
                .map(|_| on_chain_account.into())
        } else {
            // Simply re-generate the account and insert it, without fetching any data
            client
                .add_account(
                    &generated_acct,
                    Some(account_seed),
                    &AuthSecretKey::RpoFalcon512(key_pair),
                    false,
                )
                .await
                .map_err(|err| JsValue::from_str(&format!("Failed to import account: {:?}", err)))
                .map(|_| generated_acct.into())
        }
    }
    pub async fn import_note(&mut self, note_bytes: JsValue) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_bytes_result: Vec<u8> = from_value(note_bytes).unwrap();

            let note_file =
                NoteFile::read_from_bytes(&note_bytes_result).map_err(|err| err.to_string())?;

            match client.import_note(note_file).await {
                Ok(note_id) => Ok(JsValue::from_str(note_id.to_string().as_str())),
                Err(err) => {
                    let error_message = format!("Failed to import note: {:?}", err);
                    Err(JsValue::from_str(&error_message))
                },
            }
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

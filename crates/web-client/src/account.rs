use miden_objects::accounts::AccountId;
use wasm_bindgen::prelude::*;

use crate::{models::accounts::SerializedAccountHeader, WebClient};

#[wasm_bindgen]
impl WebClient {
    pub async fn get_accounts(&mut self) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let account_tuples = client.get_account_headers().await.unwrap();
            let accounts: Vec<SerializedAccountHeader> = account_tuples
                .into_iter()
                .map(|(account, _)| {
                    SerializedAccountHeader::new(
                        account.id().to_string(),
                        account.nonce().to_string(),
                        account.vault_root().to_string(),
                        account.storage_commitment().to_string(),
                        account.code_commitment().to_string(),
                    )
                })
                .collect();

            let accounts_as_js_value =
                serde_wasm_bindgen::to_value(&accounts).unwrap_or_else(|_| {
                    wasm_bindgen::throw_val(JsValue::from_str("Serialization error"))
                });

            Ok(accounts_as_js_value)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_account(&mut self, account_id: String) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let native_account_id = AccountId::from_hex(&account_id).unwrap();

            let result = client.get_account(native_account_id).await.unwrap();

            serde_wasm_bindgen::to_value(&result.0.id().to_string())
                .map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn fetch_and_cache_account_auth_by_pub_key(
        &mut self,
        account_id: String,
    ) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let _ = client
                .store()
                .fetch_and_cache_account_auth_by_pub_key(account_id)
                .await
                .unwrap();

            Ok(JsValue::from_str("Okay, it worked"))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

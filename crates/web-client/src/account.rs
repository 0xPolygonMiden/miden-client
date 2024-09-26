use miden_objects::accounts::AccountId as NativeAccountId;
use wasm_bindgen::prelude::*;

use crate::{
    models::{account::Account, account_header::AccountHeader, auth_secret_key::AuthSecretKey},
    WebClient,
};

#[wasm_bindgen]
impl WebClient {
    pub async fn get_accounts(&mut self) -> Result<Vec<AccountHeader>, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let result = client
                .get_account_headers()
                .await
                .map_err(|err| JsValue::from_str(&format!("Failed to get accounts: {}", err)))?;

            Ok(result.into_iter().map(|(header, _)| header.into()).collect())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_account(&mut self, account_id: &str) -> Result<Account, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let native_account_id = NativeAccountId::from_hex(account_id).map_err(|err| {
                JsValue::from_str(&format!("Failed to parse account ID: {:?}", err))
            })?;
            let result = client
                .get_account(native_account_id)
                .await
                .map_err(|err| JsValue::from_str(&format!("Failed to get account: {}", err)))?;

            Ok(result.0.into())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_account_auth(&mut self, account_id: &str) -> Result<AuthSecretKey, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let native_account_id = NativeAccountId::from_hex(account_id).map_err(|err| {
                JsValue::from_str(&format!("Failed to parse account ID: {:?}", err))
            })?;
            let native_auth_secret_key =
                client.get_account_auth(native_account_id).await.map_err(|err| {
                    JsValue::from_str(&format!("Failed to get account auth: {}", err))
                })?;

            Ok(native_auth_secret_key.into())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn fetch_and_cache_account_auth_by_pub_key(
        &mut self,
        account_id: &str,
    ) -> Result<AuthSecretKey, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let native_auth_secret_key = client
                .store()
                .fetch_and_cache_account_auth_by_pub_key(account_id)
                .await
                .map_err(|err| {
                    JsValue::from_str(&format!("Failed to fetch and cache account auth: {}", err))
                })?;

            Ok(native_auth_secret_key.into())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

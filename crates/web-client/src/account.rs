use miden_client::store::AccountRecord;
use miden_objects::account::Account as NativeAccount;
use wasm_bindgen::prelude::*;

use crate::{
    models::{account::Account, account_header::AccountHeader, account_id::AccountId},
    WebClient,
};

#[wasm_bindgen]
impl WebClient {
    pub async fn get_accounts(&mut self) -> Result<Vec<AccountHeader>, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let result = client
                .get_account_headers()
                .await
                .map_err(|err| JsValue::from_str(&format!("Failed to get accounts: {err}")))?;

            Ok(result.into_iter().map(|(header, _)| header.into()).collect())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_account(
        &mut self,
        account_id: &AccountId,
    ) -> Result<Option<Account>, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let result = client
                .get_account(account_id.into())
                .await
                .map_err(|err| JsValue::from_str(&format!("Failed to get account: {err}")))?;
            let account: Option<NativeAccount> = result.map(AccountRecord::into);

            Ok(account.map(miden_client::account::Account::into))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

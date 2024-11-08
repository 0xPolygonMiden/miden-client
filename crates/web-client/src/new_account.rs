use miden_client::accounts::AccountTemplate;
use miden_objects::assets::TokenSymbol;
use wasm_bindgen::prelude::*;

use super::models::{account::Account, account_storage_mode::AccountStorageMode};
use crate::WebClient;

#[wasm_bindgen]
impl WebClient {
    pub async fn new_wallet(
        &mut self,
        storage_mode: &AccountStorageMode,
        mutable: bool,
    ) -> Result<Account, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let client_template = AccountTemplate::BasicWallet {
                mutable_code: mutable,
                storage_mode: storage_mode.into(),
            };
            match client.new_account(client_template).await {
                Ok((native_account, _)) => Ok(native_account.into()),
                Err(err) => {
                    let error_message = format!("Failed to create new wallet: {:?}", err);
                    Err(JsValue::from_str(&error_message))
                },
            }
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn new_faucet(
        &mut self,
        storage_mode: &AccountStorageMode,
        non_fungible: bool,
        token_symbol: &str,
        decimals: u8,
        max_supply: u64,
    ) -> Result<Account, JsValue> {
        if non_fungible {
            return Err(JsValue::from_str("Non-fungible faucets are not supported yet"));
        }

        if let Some(client) = self.get_mut_inner() {
            let client_template = AccountTemplate::FungibleFaucet {
                token_symbol: TokenSymbol::new(token_symbol)
                    .map_err(|e| JsValue::from_str(&e.to_string()))?,
                decimals,
                max_supply,
                storage_mode: storage_mode.into(),
            };

            match client.new_account(client_template).await {
                Ok((native_account, _)) => Ok(native_account.into()),
                Err(err) => {
                    let error_message = format!("Failed to create new faucet: {:?}", err);
                    Err(JsValue::from_str(&error_message))
                },
            }
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

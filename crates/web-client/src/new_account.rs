use std::str::FromStr;

use miden_client::accounts::AccountTemplate;
use miden_objects::{accounts::AccountStorageMode, assets::TokenSymbol};
use wasm_bindgen::prelude::*;

use crate::WebClient;

#[wasm_bindgen]
impl WebClient {
    pub async fn new_wallet(
        &mut self,
        storage_type: String,
        mutable: bool,
    ) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let client_template = AccountTemplate::BasicWallet {
                mutable_code: mutable,
                storage_type: AccountStorageMode::try_from(storage_type.as_str())
                    .map_err(|_| JsValue::from_str("Invalid storage mode"))?,
            };

            match client.new_account(client_template).await {
                Ok((account, _)) => serde_wasm_bindgen::to_value(&account.id().to_string())
                    .map_err(|e| JsValue::from_str(&e.to_string())),
                Err(err) => {
                    let error_message = format!("Failed to create new account: {:?}", err);
                    Err(JsValue::from_str(&error_message))
                },
            }
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn new_faucet(
        &mut self,
        storage_type: String,
        non_fungible: bool,
        token_symbol: String,
        decimals: String,
        max_supply: String,
    ) -> Result<JsValue, JsValue> {
        if non_fungible {
            return Err(JsValue::from_str("Non-fungible faucets are not supported yet"));
        }

        if let Some(client) = self.get_mut_inner() {
            let client_template = AccountTemplate::FungibleFaucet {
                token_symbol: TokenSymbol::new(&token_symbol)
                    .map_err(|e| JsValue::from_str(&e.to_string()))?,
                decimals: decimals.parse::<u8>().map_err(|e| JsValue::from_str(&e.to_string()))?,
                max_supply: max_supply
                    .parse::<u64>()
                    .map_err(|e| JsValue::from_str(&e.to_string()))?,
                storage_type: AccountStorageMode::from_str(&storage_type)
                    .map_err(|_| JsValue::from_str("Invalid storage mode"))?,
            };

            match client.new_account(client_template).await {
                Ok((account, _)) => serde_wasm_bindgen::to_value(&account.id().to_string())
                    .map_err(|e| JsValue::from_str(&e.to_string())),
                Err(err) => {
                    let error_message = format!("Failed to create new account: {:?}", err);
                    Err(JsValue::from_str(&error_message))
                },
            }
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

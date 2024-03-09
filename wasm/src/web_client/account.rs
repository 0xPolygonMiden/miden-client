use super::WebClient;

use crate::native_code::accounts;
use crate::native_code::rpc::NodeRpcClient;
use crate::native_code::store::Store;

use serde::{Serialize, Deserialize};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::*;
use web_sys::console;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AccountTemplate {
    BasicImmutable,
    BasicMutable,
    FungibleFaucet {
        //token_symbol: TokenSymbol,
        decimals: u8,
        max_supply: u64
    },
    NonFungibleFaucet,
}

// Account functions to be exposed to the JavaScript environment
// For now, just a simple function that calls an underlying store method
// and inserts a string to the indexedDB store. Also tests out a simple
// RPC call. 
#[wasm_bindgen]
impl WebClient {
    pub async fn fake_new_account(&mut self) -> Result<JsValue, JsValue> {
        if let Some(ref mut client) = self.get_mut_inner() {
            let _ = client.store.insert_string("New account created".to_string()).await
                .map(|_| JsValue::from_str("Account created successfully"))
                .map_err(|_| JsValue::from_str("Failed to create new account"));

            client.rpc_api.test_rpc().await // This is the new line
                .map(|_| JsValue::from_str("RPC call successful"))
                .map_err(|_| JsValue::from_str("RPC call failed"))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn new_account(
        &mut self,
        template: JsValue
    ) -> () {
        if let Some(ref mut client) = self.get_mut_inner() {
            let account_template_result: Result<AccountTemplate, _> = from_value(template);
            match account_template_result {
                Ok(account_template) => {
                    let client_template = match account_template {
                        AccountTemplate::BasicImmutable => accounts::AccountTemplate::BasicWallet {
                            mutable_code: false,
                            storage_mode: accounts::AccountStorageMode::Local,
                        },
                        AccountTemplate::BasicMutable => accounts::AccountTemplate::BasicWallet {
                            mutable_code: true,
                            storage_mode: accounts::AccountStorageMode::Local,
                        },
                        AccountTemplate::FungibleFaucet {
                            //token_symbol,
                            decimals,
                            max_supply,
                        } => accounts::AccountTemplate::FungibleFaucet {
                            // token_symbol: TokenSymbol::new(token_symbol)
                            //     .map_err(|err| format!("error: token symbol is invalid: {}", err))?,
                            decimals: decimals,
                            max_supply: max_supply,
                            storage_mode: accounts::AccountStorageMode::Local,
                        },
                        AccountTemplate::NonFungibleFaucet => todo!(),
                    };
        
                    // TODO: uncomment this when the Falcon library Rust implementation
                    // is completed by the miden team
        
                    // match client.new_account(client_template).await {
                    //     Ok((account, word)) => {
                    //         // Create a struct or tuple to hold both values
                    //         let result = (account, word);
                    //         // Convert directly to JsValue
                    //         serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
                    //     },
                    //     Err(err) => Err(JsValue::from_str(&err)),
                    // }
        
                    // TODO: remove this when the Falcon library Rust implementation
                    // is completed by the miden team
                    
                    let message = client.new_account(client_template).await;
                    let js_value_message = JsValue::from_str(&message);
                    
                    // Print the message to the Chrome console
                    console::log_1(&js_value_message);
                },
                Err(e) => {
                    // Error handling: log the error message to the browser's console
                    let error_message = format!("Failed to parse AccountTemplate: {:?}", e);
                    console::error_1(&error_message.into());
                }
            }
        } else {
            console::error_1(&"Client not initialized".into());
        }
    }
}
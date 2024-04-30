use super::WebClient;

use base64::encode;
use miden_objects::{accounts::{AccountData, AccountId}, assets::TokenSymbol, notes::NoteId};
use miden_tx::utils::{Deserializable, Serializable};

use crate::native_code::accounts;
use crate::native_code::rpc::NodeRpcClient;
use crate::native_code::store::Store;
use crate::native_code::store::AuthInfo;

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
        token_symbol: String,
        decimals: String, // u8
        max_supply: String // u64
    },
    NonFungibleFaucet,
}

// Account functions to be exposed to the JavaScript environment
// For now, just a simple function that calls an underlying store method
// and inserts a string to the indexedDB store. Also tests out a simple
// RPC call. 
#[wasm_bindgen]
impl WebClient {
    pub async fn test_store_and_rpc(&mut self) -> Result<JsValue, JsValue> {
        if let Some(ref mut client) = self.get_mut_inner() {
            let _ = client.store.insert_string("Test string".to_string()).await
                .map(|_| JsValue::from_str("Test string inserted successfully"))
                .map_err(|_| JsValue::from_str("Failed to insert test string"));

            client.rpc_api.test_rpc().await // This is the new line
                .map(|_| JsValue::from_str("RPC call successful"))
                .map_err(|_| JsValue::from_str("RPC call failed"))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn import_account(
        &mut self,
        account_bytes: JsValue
    ) -> Result<JsValue, JsValue> {
        if let Some(ref mut client) = self.get_mut_inner() {
            let account_bytes_result: Vec<u8> = from_value(account_bytes).unwrap();
            let account_data = AccountData::read_from_bytes(&account_bytes_result).map_err(|err| err.to_string())?;
            let account_id = account_data.account.id().to_string();

            match client.import_account(account_data).await {
                Ok(_) => {
                    let message = format!("Import account with ID: {}", account_id);
                    Ok(JsValue::from_str(&message))
                },
                Err(err) => {
                    let error_message = format!("Failed to import account: {:?}", err);
                    Err(JsValue::from_str(&error_message))
                
                }
            }
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn new_account(
        &mut self,
        template: JsValue
    ) -> Result<JsValue, JsValue> {
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
                            token_symbol,
                            decimals,
                            max_supply,
                        } => accounts::AccountTemplate::FungibleFaucet {
                            token_symbol: TokenSymbol::new(&token_symbol).unwrap(),
                            decimals: decimals.parse::<u8>().unwrap(),
                            max_supply: max_supply.parse::<u64>().unwrap(),
                            storage_mode: accounts::AccountStorageMode::Local,
                        },
                        AccountTemplate::NonFungibleFaucet => todo!(),
                    };

                    match client.new_account(client_template).await {
                        Ok((account, word)) => {
                            // Create a struct or tuple to hold both values
                            let result = (account, word);
                            // Convert directly to JsValue
                            serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
                        },
                        Err(err) => {
                            let error_message = format!("Failed to create new account: {:?}", err);
                            Err(JsValue::from_str(&error_message))
                        }
                    }
                },
                Err(e) => {
                    let error_message = format!("Failed to parse AccountTemplate: {:?}", e);
                    Err(JsValue::from_str(&error_message))
                }
            }
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_accounts(
        &mut self
    ) -> Result<JsValue, JsValue> {
        if let Some(ref mut client) = self.get_mut_inner() {
            let account_tuples = client.get_accounts().await.unwrap();
            let accounts: Vec<String> = account_tuples.into_iter().map(|(account, word)| {
                let word = word.map_or("No word".to_string(), |w| w[0].to_string());
                format!("ID: {}, Word: {}", account.id().to_string(), word)
            }).collect();

            // Convert the Vec<String> to JsValue
            serde_wasm_bindgen::to_value(&accounts).map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_account(
        &mut self,
        account_id: String
    ) -> Result<JsValue, JsValue> {
        if let Some(ref mut client) = self.get_mut_inner() {
            let native_account_id = AccountId::from_hex(&account_id).unwrap();

            let result = client.get_account(native_account_id).await.unwrap();

            let word = result.1.map_or("No word".to_string(), |w| w[0].to_string());
            Ok(JsValue::from_str(&format!("ID: {}, Word: {}", result.0.id().to_string(), word)))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_account_stub_by_id(
        &mut self,
        account_id: String
    ) -> Result<JsValue, JsValue> {
        if let Some(ref mut client) = self.get_mut_inner() {
            let native_account_id = AccountId::from_hex(&account_id).unwrap();

            let result = client.get_account_stub_by_id(native_account_id).await.unwrap();
            
            let word = result.1.map_or("No word".to_string(), |w| w[0].to_string());
            Ok(JsValue::from_str(&format!("ID: {}, Word: {}", result.0.id().to_string(), word)))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_account_auth(
        &mut self,
        account_id: String
    ) -> Result<JsValue, JsValue> {
        if let Some(ref mut client) = self.get_mut_inner() {
            let native_account_id = AccountId::from_hex(&account_id).unwrap();

            let result = client.get_account_auth(native_account_id).await.unwrap();
            let mut bytes = Vec::new();
            result.write_into(&mut bytes);
            let base64_encoded = encode(&bytes);
            Ok(JsValue::from_str(&base64_encoded))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}
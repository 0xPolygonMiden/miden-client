use super::WebClient;

use crate::native_code::store::NativeTransactionFilter;

use serde::{Serialize, Deserialize};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::*;
use web_sys::console;

#[derive(Serialize, Deserialize)]
pub enum TransactionFilter {
    All,
    Uncomitted
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TransactionType {
    /// Create a Pay To ID transaction.
    P2ID {
        sender_account_id: String,
        target_account_id: String,
        faucet_id: String,
        amount: u64,
    },
    /// Mint `amount` tokens from the specified fungible faucet (corresponding to `faucet_id`). The created note can then be then consumed by
    /// `target_account_id`.
    Mint {
        target_account_id: String,
        faucet_id: String,
        amount: u64,
    },
    /// Create a Pay To ID with Recall transaction.
    P2IDR,
    /// Consume with the account corresponding to `account_id` all of the notes from `list_of_notes`.
    ConsumeNotes {
        account_id: String,
        /// A list of note IDs or the hex prefixes of their corresponding IDs
        list_of_notes: Vec<String>,
    },
}

#[wasm_bindgen]
impl WebClient {
    pub async fn get_transactions(
        &mut self,
        filter: JsValue
    ) -> () {
        if let Some(ref mut client) = self.get_mut_inner() {
            let filter: TransactionFilter = from_value(filter).unwrap();
            let native_filter = match filter {
                TransactionFilter::All => NativeTransactionFilter::All,
                TransactionFilter::Uncomitted => NativeTransactionFilter::Uncomitted
            };

            let message = client.get_transactions(native_filter).await;
            let js_value_message = JsValue::from_str(&message);
            
            // Print the message to the Chrome console
            console::log_1(&js_value_message);
        } else {
            console::error_1(&"Client not initialized".into());
        }
    }

    pub async fn new_transaction(
        &mut self,
        //transaction_type: JsValue
    ) -> () {
        if let Some(ref mut client) = self.get_mut_inner() {
            //let transaction_type: TransactionType = from_value(transaction_type).unwrap();
            //let transaction_template = build_transaction_template(client, transaction_type)?;

            let message = client.new_transaction().await;
            let js_value_message = JsValue::from_str(&message);
            
            // Print the message to the Chrome console
            console::log_1(&js_value_message);

            let message = client.send_transaction().await;
            let js_value_message = JsValue::from_str(&message);
            
            // Print the message to the Chrome console
            console::log_1(&js_value_message);
        } else {
            console::error_1(&"Client not initialized".into());
        }
    }
}
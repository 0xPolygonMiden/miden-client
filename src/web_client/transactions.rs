use wasm_bindgen::prelude::*;

use super::WebClient;
use crate::{client::transactions::TransactionRecord, store::TransactionFilter};

#[wasm_bindgen]
impl WebClient {
    pub async fn get_transactions(&mut self) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let transactions: Vec<TransactionRecord> = client
                .get_transactions(TransactionFilter::All)
                .await
                .map_err(|e| JsValue::from_str(&format!("Error fetching transactions: {:?}", e)))?;

            let transaction_ids: Vec<String> =
                transactions.iter().map(|transaction| transaction.id.to_string()).collect();

            serde_wasm_bindgen::to_value(&transaction_ids)
                .map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

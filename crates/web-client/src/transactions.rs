use miden_client::transaction::{
    TransactionRecord as NativeTransactionRecord, TransactionScript as NativeTransactionScript,
};
use wasm_bindgen::prelude::*;

use super::models::{
    transaction_filter::TransactionFilter, transaction_record::TransactionRecord,
    transaction_script::TransactionScript,
};
use crate::WebClient;

#[wasm_bindgen]
impl WebClient {
    #[wasm_bindgen(js_name = "getTransactions")]
    pub async fn get_transactions(
        &mut self,
        transaction_filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let transaction_records: Vec<NativeTransactionRecord> = client
                .get_transactions(transaction_filter.into())
                .await
                .map_err(|err| JsValue::from_str(&format!("Failed to get transactions: {err}")))?;

            Ok(transaction_records.into_iter().map(Into::into).collect())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    #[wasm_bindgen(js_name = "compileTxScript")]
    pub fn compile_tx_script(&mut self, script: &str) -> Result<TransactionScript, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let native_tx_script: NativeTransactionScript =
                client.compile_tx_script(vec![], script).map_err(|err| {
                    JsValue::from_str(&format!("Failed to compile transaction script: {err}"))
                })?;
            Ok(native_tx_script.into())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

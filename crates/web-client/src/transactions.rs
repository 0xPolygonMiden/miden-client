use miden_client::{
    store::TransactionFilter,
    transactions::{TransactionRecord, TransactionScript as NativeTransactionScript},
};
use miden_objects::{
    Felt as NativeFelt,
    Word as NativeWord
};
use wasm_bindgen::prelude::*;

use super::models::{
    transaction_script::TransactionScript, transaction_script_inputs::{TransactionScriptInputPair, TransactionScriptInputPairArray},
};
use crate::WebClient;

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

    pub async fn compile_tx_script(
        &mut self,
        script: &str,
        transaction_script_input_pairs: &TransactionScriptInputPairArray,
    ) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let native_input_pairs: Vec<(NativeWord, Vec<NativeFelt>)> = transaction_script_input_pairs.into();
            let native_tx_script: NativeTransactionScript =
                client.compile_tx_script(native_input_pairs, script).unwrap();
            let tx_script: TransactionScript =
                TransactionScript::from_native_transaction_script(native_tx_script);
            let tx_script_as_js_value = JsValue::from(tx_script);
            Ok(tx_script_as_js_value)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

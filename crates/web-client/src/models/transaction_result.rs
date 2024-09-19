use miden_client::transactions::TransactionResult as NativeTransactionResult;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct TransactionResult(NativeTransactionResult);

#[wasm_bindgen]
impl TransactionResult {
    pub(crate) fn from_native_transaction_result(
        native_transaction_result: NativeTransactionResult,
    ) -> TransactionResult {
        TransactionResult(native_transaction_result)
    }
}

// CONVERSIONS
// ================================================================================================

impl From<TransactionResult> for NativeTransactionResult {
    fn from(transaction_result: TransactionResult) -> Self {
        transaction_result.0
    }
}

impl From<&TransactionResult> for NativeTransactionResult {
    fn from(transaction_result: &TransactionResult) -> Self {
        transaction_result.0.clone()
    }
}

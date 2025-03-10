use miden_client::transaction::TransactionResult as NativeTransactionResult;
use wasm_bindgen::prelude::*;

use super::{
    account_delta::AccountDelta, executed_transaction::ExecutedTransaction,
    input_notes::InputNotes, output_notes::OutputNotes, transaction_args::TransactionArgs,
};

#[wasm_bindgen]
pub struct TransactionResult(NativeTransactionResult);

#[wasm_bindgen]
impl TransactionResult {
    #[wasm_bindgen(js_name = "executedTransaction")]
    pub fn executed_transaction(&self) -> ExecutedTransaction {
        self.0.executed_transaction().into()
    }

    #[wasm_bindgen(js_name = "createdNotes")]
    pub fn created_notes(&self) -> OutputNotes {
        self.0.created_notes().into()
    }

    // TODO: relevant_notes

    #[wasm_bindgen(js_name = "blockNum")]
    pub fn block_num(&self) -> u32 {
        self.0.block_num().as_u32()
    }

    #[wasm_bindgen(js_name = "transactionArguments")]
    pub fn transaction_arguments(&self) -> TransactionArgs {
        self.0.transaction_arguments().into()
    }

    #[wasm_bindgen(js_name = "accountDelta")]
    pub fn account_delta(&self) -> AccountDelta {
        self.0.account_delta().into()
    }

    #[wasm_bindgen(js_name = "consumedNotes")]
    pub fn consumed_notes(&self) -> InputNotes {
        self.0.consumed_notes().into()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeTransactionResult> for TransactionResult {
    fn from(native_transaction_result: NativeTransactionResult) -> Self {
        TransactionResult(native_transaction_result)
    }
}

impl From<&NativeTransactionResult> for TransactionResult {
    fn from(native_transaction_result: &NativeTransactionResult) -> Self {
        TransactionResult(native_transaction_result.clone())
    }
}

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

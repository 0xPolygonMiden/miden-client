use miden_client::transactions::TransactionResult as NativeTransactionResult;
use wasm_bindgen::prelude::*;

use super::{
    account_delta::AccountDelta, executed_transaction::ExecutedTransaction,
    input_notes::InputNotes, output_notes::OutputNotes, transaction_args::TransactionArgs,
};

#[wasm_bindgen]
pub struct TransactionResult(NativeTransactionResult);

#[wasm_bindgen]
impl TransactionResult {
    pub fn executed_transaction(&self) -> ExecutedTransaction {
        self.0.executed_transaction().into()
    }

    pub fn created_notes(&self) -> OutputNotes {
        self.0.created_notes().into()
    }

    // TODO: relevant_notes

    pub fn block_num(&self) -> u32 {
        self.0.block_num()
    }

    pub fn transaction_arguments(&self) -> TransactionArgs {
        self.0.transaction_arguments().into()
    }

    pub fn account_delta(&self) -> AccountDelta {
        self.0.account_delta().into()
    }

    pub fn consumed_notes(&self) -> InputNotes {
        self.0.consumed_notes().into()
    }

    pub(crate) fn from_native_transaction_result(
        native_transaction_result: NativeTransactionResult,
    ) -> TransactionResult {
        TransactionResult(native_transaction_result)
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

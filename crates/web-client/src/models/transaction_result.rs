use miden_client::transaction::TransactionResult as NativeTransactionResult;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::Uint8Array;

use crate::{
    models::{
        account_delta::AccountDelta, executed_transaction::ExecutedTransaction,
        input_notes::InputNotes, output_notes::OutputNotes, transaction_args::TransactionArgs,
    },
    utils::*,
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
        self.0.block_num().as_u32()
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

    pub fn serialize(&self) -> Uint8Array {
        serialize_to_uint8array(&self.0)
    }

    pub fn deserialize(bytes: Uint8Array) -> Result<TransactionResult, JsValue> {
        deserialize_from_uint8array::<NativeTransactionResult>(bytes).map(TransactionResult)
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

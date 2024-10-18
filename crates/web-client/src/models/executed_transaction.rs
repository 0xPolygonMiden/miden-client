use miden_objects::transaction::ExecutedTransaction as NativeExecutedTransaction;
use wasm_bindgen::prelude::*;

use super::{
    account::Account, account_delta::AccountDelta, account_header::AccountHeader,
    account_id::AccountId, block_header::BlockHeader, input_notes::InputNotes,
    output_notes::OutputNotes, transaction_args::TransactionArgs, transaction_id::TransactionId,
};

#[derive(Clone)]
#[wasm_bindgen]
pub struct ExecutedTransaction(NativeExecutedTransaction);

#[wasm_bindgen]
impl ExecutedTransaction {
    pub fn id(&self) -> TransactionId {
        self.0.id().into()
    }

    pub fn account_id(&self) -> AccountId {
        self.0.account_id().into()
    }

    pub fn initial_account(&self) -> Account {
        self.0.initial_account().into()
    }

    pub fn final_account(&self) -> AccountHeader {
        self.0.final_account().into()
    }

    pub fn input_notes(&self) -> InputNotes {
        self.0.input_notes().into()
    }

    pub fn output_notes(&self) -> OutputNotes {
        self.0.output_notes().into()
    }

    pub fn tx_args(&self) -> TransactionArgs {
        self.0.tx_args().into()
    }

    pub fn block_header(&self) -> BlockHeader {
        self.0.block_header().into()
    }

    pub fn account_delta(&self) -> AccountDelta {
        self.0.account_delta().into()
    }

    // TODO: tx_inputs

    // TODO: advice_witness
}

// CONVERSIONS
// ================================================================================================

impl From<NativeExecutedTransaction> for ExecutedTransaction {
    fn from(native_executed_transaction: NativeExecutedTransaction) -> Self {
        ExecutedTransaction(native_executed_transaction)
    }
}

impl From<&NativeExecutedTransaction> for ExecutedTransaction {
    fn from(native_executed_transaction: &NativeExecutedTransaction) -> Self {
        ExecutedTransaction(native_executed_transaction.clone())
    }
}

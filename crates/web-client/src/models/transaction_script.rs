use miden_objects::transaction::TransactionScript as NativeTransactionScript;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionScript(NativeTransactionScript);

impl TransactionScript {
    pub(crate) fn from_native_transaction_script(
        native_transaction_script: NativeTransactionScript,
    ) -> TransactionScript {
        TransactionScript(native_transaction_script)
    }
}

// CONVERSIONS
// ================================================================================================

impl From<TransactionScript> for NativeTransactionScript {
    fn from(transaction_script: TransactionScript) -> Self {
        transaction_script.0
    }
}

impl From<&TransactionScript> for NativeTransactionScript {
    fn from(transaction_script: &TransactionScript) -> Self {
        transaction_script.0.clone()
    }
}

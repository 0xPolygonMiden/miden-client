use miden_objects::transaction::TransactionScript as NativeTransactionScript;
use wasm_bindgen::prelude::*;

use super::rpo_digest::RpoDigest;

#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionScript(NativeTransactionScript);

#[wasm_bindgen]
impl TransactionScript {
    pub fn hash(&self) -> RpoDigest {
        self.0.hash().into()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeTransactionScript> for TransactionScript {
    fn from(native_transaction_script: NativeTransactionScript) -> Self {
        TransactionScript(native_transaction_script)
    }
}

impl From<&NativeTransactionScript> for TransactionScript {
    fn from(native_transaction_script: &NativeTransactionScript) -> Self {
        TransactionScript(native_transaction_script.clone())
    }
}

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

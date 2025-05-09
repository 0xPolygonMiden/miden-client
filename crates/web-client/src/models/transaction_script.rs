use miden_objects::transaction::TransactionScript as NativeTransactionScript;
use wasm_bindgen::prelude::*;

use crate::models::{assembler::Assembler, transaction_script_inputs::TransactionScriptInputPairArray};

use super::rpo_digest::RpoDigest;

#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionScript(NativeTransactionScript);

#[wasm_bindgen]
impl TransactionScript {
    pub fn root(&self) -> RpoDigest {
        self.0.root().into()
    }

    pub fn compile(
        script_code: &str,
        inputs: TransactionScriptInputPairArray,
        assembler: &Assembler
    ) -> TransactionScript {
        let native_script = NativeTransactionScript::compile(script_code, inputs.into(), assembler.into());
        TransactionScript(native_script)
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

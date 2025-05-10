use miden_objects::{
    Felt as NativeFelt, Word as NativeWord,
    transaction::TransactionScript as NativeTransactionScript,
};
use wasm_bindgen::prelude::*;

use super::rpo_digest::RpoDigest;
use crate::models::{
    assembler::Assembler, transaction_script_inputs::TransactionScriptInputPairArray,
};

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
        assembler: &Assembler,
    ) -> Result<TransactionScript, JsValue> {
        let native_inputs: Vec<(NativeWord, Vec<NativeFelt>)> = inputs.into();

        let native = NativeTransactionScript::compile(
            script_code,
            native_inputs, // now the compiler knows this is a Vec<…>
            assembler.into(),
        )
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(TransactionScript(native))
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

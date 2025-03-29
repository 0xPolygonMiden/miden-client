use miden_objects::transaction::TransactionArgs as NativeTransactionArgs;
use wasm_bindgen::prelude::*;

use super::{
    advice_inputs::AdviceInputs, note_id::NoteId, transaction_script::TransactionScript, word::Word,
};

#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionArgs(NativeTransactionArgs);

#[wasm_bindgen]
impl TransactionArgs {
    #[wasm_bindgen(js_name = "txScript")]
    pub fn tx_script(&self) -> Option<TransactionScript> {
        self.0.tx_script().map(Into::into)
    }

    #[wasm_bindgen(js_name = "getNoteArgs")]
    pub fn get_note_args(&self, note_id: &NoteId) -> Option<Word> {
        self.0.get_note_args(note_id.into()).map(Into::into)
    }

    #[wasm_bindgen(js_name = "adviceInputs")]
    pub fn advice_inputs(&self) -> AdviceInputs {
        self.0.advice_inputs().into()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeTransactionArgs> for TransactionArgs {
    fn from(native_args: NativeTransactionArgs) -> Self {
        TransactionArgs(native_args)
    }
}

impl From<&NativeTransactionArgs> for TransactionArgs {
    fn from(native_args: &NativeTransactionArgs) -> Self {
        TransactionArgs(native_args.clone())
    }
}

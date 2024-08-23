use miden_objects::{Felt as NativeFelt, Word as NativeWord};
use wasm_bindgen::prelude::*;

use super::{felt::Felt, word::Word};

#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionScriptInputPair {
    word: Word,
    felts: Vec<Felt>,
}

#[wasm_bindgen]
impl TransactionScriptInputPair {
    #[wasm_bindgen(constructor)]
    pub fn new(word: Word, felts: Vec<Felt>) -> TransactionScriptInputPair {
        TransactionScriptInputPair { word, felts }
    }

    pub fn word(&self) -> Word {
        self.word.clone()
    }

    pub fn felts(&self) -> Vec<Felt> {
        self.felts.clone()
    }
}

impl From<TransactionScriptInputPair> for (NativeWord, Vec<NativeFelt>) {
    fn from(transaction_script_input_pair: TransactionScriptInputPair) -> Self {
        let native_word: NativeWord = transaction_script_input_pair.word.into();
        let native_felts: Vec<NativeFelt> = transaction_script_input_pair
            .felts
            .into_iter()
            .map(|felt| felt.into())
            .collect();
        (native_word, native_felts)
    }
}

impl From<&TransactionScriptInputPair> for (NativeWord, Vec<NativeFelt>) {
    fn from(transaction_script_input_pair: &TransactionScriptInputPair) -> Self {
        let native_word: NativeWord = transaction_script_input_pair.word.clone().into();
        let native_felts: Vec<NativeFelt> = transaction_script_input_pair
            .felts
            .iter()
            .map(|felt| felt.clone().into())
            .collect();
        (native_word, native_felts)
    }
}

#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionScriptInputPairArray(Vec<TransactionScriptInputPair>);

#[wasm_bindgen]
impl TransactionScriptInputPairArray {
    #[wasm_bindgen(constructor)]
    pub fn new(transaction_script_input_pairs: Option<Vec<TransactionScriptInputPair>>) -> TransactionScriptInputPairArray {
        let transaction_script_input_pairs = transaction_script_input_pairs.unwrap_or_default();
        TransactionScriptInputPairArray(transaction_script_input_pairs)
    }

    pub fn push(&mut self, transaction_script_input_pair: &TransactionScriptInputPair) {
        self.0.push(transaction_script_input_pair.clone());
    }
}

impl From<TransactionScriptInputPairArray> for Vec<(NativeWord, Vec<NativeFelt>)> {
    fn from(transaction_script_input_pair_array: TransactionScriptInputPairArray) -> Self {
        transaction_script_input_pair_array
            .0
            .into_iter()
            .map(|transaction_script_input_pair| transaction_script_input_pair.into())
            .collect()
    }
}

impl From<&TransactionScriptInputPairArray> for Vec<(NativeWord, Vec<NativeFelt>)> {
    fn from(transaction_script_input_pair_array: &TransactionScriptInputPairArray) -> Self {
        transaction_script_input_pair_array
            .0
            .iter()
            .map(|transaction_script_input_pair| transaction_script_input_pair.into())
            .collect()
    }
}

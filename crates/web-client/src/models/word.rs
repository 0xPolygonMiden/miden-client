use miden_objects::{Felt as NativeFelt, Word as NativeWord};
use wasm_bindgen::prelude::*;

use super::felt::Felt;

#[wasm_bindgen]
#[derive(Clone)]
pub struct Word(NativeWord);

#[wasm_bindgen]
impl Word {
    #[wasm_bindgen(js_name = "newFromU64s")]
    pub fn new_from_u64s(u64_vec: Vec<u64>) -> Word {
        let fixed_array_u64: [u64; 4] = u64_vec.try_into().unwrap();

        let native_felt_vec: [NativeFelt; 4] = fixed_array_u64
            .iter()
            .map(|&v| NativeFelt::new(v))
            .collect::<Vec<NativeFelt>>()
            .try_into()
            .unwrap();

        let native_word: NativeWord = native_felt_vec;

        Word(native_word)
    }

    #[wasm_bindgen(js_name = "newFromFelts")]
    #[allow(clippy::needless_pass_by_value)]
    pub fn new_from_felts(felt_vec: Vec<Felt>) -> Word {
        let native_felt_vec: [NativeFelt; 4] = felt_vec
            .iter()
            .map(|felt: &Felt| felt.into())
            .collect::<Vec<NativeFelt>>()
            .try_into()
            .unwrap();

        let native_word: NativeWord = native_felt_vec;

        Word(native_word)
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeWord> for Word {
    fn from(native_word: NativeWord) -> Self {
        Word(native_word)
    }
}

impl From<&NativeWord> for Word {
    fn from(native_word: &NativeWord) -> Self {
        Word(*native_word)
    }
}

impl From<Word> for NativeWord {
    fn from(word: Word) -> Self {
        word.0
    }
}

impl From<&Word> for NativeWord {
    fn from(word: &Word) -> Self {
        word.0
    }
}

use miden_objects::{
    Felt as NativeFelt, Word as NativeWord, crypto::hash::rpo::RpoDigest as NativeRpoDigest,
};
use wasm_bindgen::prelude::*;

use super::{felt::Felt, word::Word};

#[derive(Clone)]
#[wasm_bindgen]
pub struct RpoDigest(NativeRpoDigest);

#[wasm_bindgen]
impl RpoDigest {
    #[wasm_bindgen(constructor)]
    pub fn new(value: Vec<Felt>) -> RpoDigest {
        let native_felts: [NativeFelt; 4] = value
            .into_iter()
            .map(|felt: Felt| felt.into())
            .collect::<Vec<NativeFelt>>()
            .try_into()
            .unwrap();

        RpoDigest(NativeRpoDigest::new(native_felts))
    }

    #[wasm_bindgen(js_name = "toWord")]
    pub fn to_word(&self) -> Word {
        let native_word: NativeWord = self.0.into();
        native_word.into()
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> String {
        self.0.to_hex()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeRpoDigest> for RpoDigest {
    fn from(native_rpo_digest: NativeRpoDigest) -> Self {
        RpoDigest(native_rpo_digest)
    }
}

impl From<&NativeRpoDigest> for RpoDigest {
    fn from(native_rpo_digest: &NativeRpoDigest) -> Self {
        RpoDigest(*native_rpo_digest)
    }
}

impl From<RpoDigest> for NativeRpoDigest {
    fn from(rpo_digest: RpoDigest) -> Self {
        rpo_digest.0
    }
}

impl From<&RpoDigest> for NativeRpoDigest {
    fn from(rpo_digest: &RpoDigest) -> Self {
        rpo_digest.0
    }
}

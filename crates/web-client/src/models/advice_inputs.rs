use miden_objects::vm::AdviceInputs as NativeAdviceInputs;
use wasm_bindgen::prelude::*;

use super::{felt::Felt, rpo_digest::RpoDigest};

#[derive(Clone)]
#[wasm_bindgen]
pub struct AdviceInputs(NativeAdviceInputs);

#[wasm_bindgen]
impl AdviceInputs {
    // TODO: Constructors

    // TODO: Public Mutators

    // TODO: Destructors

    pub fn stack(&self) -> Vec<Felt> {
        self.0.stack().iter().map(Into::into).collect()
    }

    pub fn mapped_values(&self, key: &RpoDigest) -> Option<Vec<Felt>> {
        self.0
            .mapped_values(&key.into())
            .map(|values| values.iter().map(Into::into).collect())
    }

    // TODO: Merkle Store
}

// CONVERSIONS
// ================================================================================================

impl From<NativeAdviceInputs> for AdviceInputs {
    fn from(native_advice_inputs: NativeAdviceInputs) -> Self {
        AdviceInputs(native_advice_inputs)
    }
}

impl From<&NativeAdviceInputs> for AdviceInputs {
    fn from(native_advice_inputs: &NativeAdviceInputs) -> Self {
        AdviceInputs(native_advice_inputs.clone())
    }
}

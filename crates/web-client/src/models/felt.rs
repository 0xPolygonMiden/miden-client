use miden_objects::Felt as NativeFelt;
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy)]
#[wasm_bindgen]
pub struct Felt(NativeFelt);

#[wasm_bindgen]
impl Felt {
    #[wasm_bindgen(constructor)]
    pub fn new(value: u64) -> Felt {
        Felt(NativeFelt::new(value))
    }

    #[wasm_bindgen(js_name = "asInt")]
    pub fn as_int(&self) -> u64 {
        self.0.as_int()
    }

    #[wasm_bindgen(js_name = "toString")]
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeFelt> for Felt {
    fn from(native_felt: NativeFelt) -> Self {
        Felt(native_felt)
    }
}

impl From<&NativeFelt> for Felt {
    fn from(native_felt: &NativeFelt) -> Self {
        Felt(*native_felt)
    }
}

impl From<Felt> for NativeFelt {
    fn from(felt: Felt) -> Self {
        felt.0
    }
}

impl From<&Felt> for NativeFelt {
    fn from(felt: &Felt) -> Self {
        felt.0
    }
}

#[derive(Clone)]
#[wasm_bindgen]
pub struct FeltArray(Vec<Felt>);

#[wasm_bindgen]
impl FeltArray {
    #[wasm_bindgen(constructor)]
    pub fn new(felts_array: Option<Vec<Felt>>) -> FeltArray {
        let felts = felts_array.unwrap_or_default();
        FeltArray(felts)
    }

    pub fn append(&mut self, felt: &Felt) {
        self.0.push(*felt);
    }
}

// CONVERSIONS
// ================================================================================================

impl From<FeltArray> for Vec<NativeFelt> {
    fn from(felt_array: FeltArray) -> Self {
        felt_array.0.into_iter().map(Into::into).collect()
    }
}

impl From<&FeltArray> for Vec<NativeFelt> {
    fn from(felt_array: &FeltArray) -> Self {
        felt_array.0.iter().map(Into::into).collect()
    }
}

impl From<FeltArray> for Vec<Felt> {
    fn from(felt_array: FeltArray) -> Self {
        felt_array.0
    }
}

impl From<&FeltArray> for Vec<Felt> {
    fn from(felt_array: &FeltArray) -> Self {
        felt_array.0.clone()
    }
}

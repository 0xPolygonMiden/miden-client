use miden_objects::assembly::{Assembler as NativeAssembler, Library as NativeLibrary};
use wasm_bindgen::prelude::*;

use crate::models::library::Library;

#[wasm_bindgen]
pub struct Assembler(NativeAssembler);

#[wasm_bindgen]
impl Assembler {
    #[wasm_bindgen(js_name = "withLibrary")]
    pub fn with_library(self, library: &Library) -> Result<Assembler, JsValue> {
        let native_lib: NativeLibrary = library.into();

        let new_native_asm = self
            .0
            .with_library(native_lib)
            .map_err(|e| JsValue::from_str(&e.to_string()))?; 

        Ok(Assembler(new_native_asm))
    }
}

// CONVERSIONS
// ================================================================================================


impl From<NativeAssembler> for Assembler {
    fn from(native_assembler: NativeAssembler) -> Self {
        Assembler(native_assembler)
    }
}

impl From<&NativeAssembler> for Assembler {
    fn from(native_assembler: &NativeAssembler) -> Self {
        Assembler(native_assembler.clone())
    }
}

impl From<Assembler> for NativeAssembler {
    fn from(assembler: Assembler) -> Self {
        assembler.0
    }
}

impl From<&Assembler> for NativeAssembler {
    fn from(assembler: &Assembler) -> Self {
        assembler.0.clone()
    }
}

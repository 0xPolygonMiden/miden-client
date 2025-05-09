use miden_objects::assembly::Assembler as NativeAssembler;
use wasm_bindgen::prelude::*;

use crate::models::library::Library;

#[wasm_bindgen]
pub struct Assembler(NativeAssembler);

#[wasm_bindgen]
impl Assembler {
    #[wasm_bindgen(js_name = "withLibrary")]
    pub fn with_library(mut self, library: &Library) -> Assembler {
        self.0 = self.0.with_library(library.clone().into());
        self
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

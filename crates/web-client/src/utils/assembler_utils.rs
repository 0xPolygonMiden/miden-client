use alloc::sync::Arc;
use miden_objects::assembly::{
    Assembler as NativeAssembler,
    DefaultSourceManager,
    LibraryPath,
    Module,
    ModuleKind,
};
use wasm_bindgen::prelude::*;

use crate::models::{assembler::Assembler, library::Library};

#[wasm_bindgen(js_name = "createAccountComponentLibrary")]
pub fn create_account_component_library(
    assembler: &Assembler,
    library_path: &str,
    source_code: &str
) -> Result<Library, JsValue> {
    let native_assembler: NativeAssembler = assembler.into();
    let source_manager = Arc::new(DefaultSourceManager::default());

    let module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new(library_path)
                .map_err(|e| JsValue::from_str(&e.to_string()))?,
            source_code,
            &source_manager,
        )
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let native_library = native_assembler
        .assemble_library([module])
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(native_library.into())
}
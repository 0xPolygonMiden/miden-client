use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{js_sys, wasm_bindgen};

#[wasm_bindgen(module = "/src/store/web_store/js/import.js")]
extern "C" {
    #[wasm_bindgen(js_name = forceImportStore)]
    pub fn idxdb_force_import_store(store_dump: JsValue) -> js_sys::Promise;

}

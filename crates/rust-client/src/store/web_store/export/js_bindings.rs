use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

#[wasm_bindgen(module = "/src/store/web_store/js/export.js")]
extern "C" {
    #[wasm_bindgen(js_name = exportStore)]
    pub fn idxdb_export_store() -> js_sys::Promise;
}

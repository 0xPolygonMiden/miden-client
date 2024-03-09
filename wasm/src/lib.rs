pub mod native_code;
pub mod web_client;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, World!");
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, World!");
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}
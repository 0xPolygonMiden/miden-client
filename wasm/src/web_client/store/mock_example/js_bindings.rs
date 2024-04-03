use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

// Example
#[wasm_bindgen(module = "/js/db/greet.js")]
extern "C" {
    #[wasm_bindgen(js_name = insertGreeting)]
    pub fn insert_greeting(greeting: String) -> js_sys::Promise;
}
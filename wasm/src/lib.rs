pub mod native_code;
pub mod web_client;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;
use web_sys::console;

#[wasm_bindgen]
extern "C" {
  fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
  unsafe { alert("Hello world from WASM!") };
}

#[wasm_bindgen]
pub fn greet2() {
  unsafe { console::log_1(&"Hello from WASM!".into()) };
}
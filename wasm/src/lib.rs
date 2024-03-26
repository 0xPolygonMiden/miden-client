pub mod native_code;
pub mod web_client;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;
use web_sys::console;

pub trait MyTrait {
  fn do_something(&self);
}

#[wasm_bindgen]
extern "C" {
  pub type JsImplementor;

  #[wasm_bindgen(method)]
  fn do_something(this: &JsImplementor);
}


#[wasm_bindgen]
pub struct MyTraitImplementor {
  js_implementor: JsImplementor,
}

#[wasm_bindgen]
impl MyTraitImplementor {
  pub fn new(js_implementor: JsImplementor) -> Self {
    Self { js_implementor }
  }
}

impl MyTrait for MyTraitImplementor {
  fn do_something(&self) {
    self.js_implementor.do_something();
  }
}

#[wasm_bindgen]
pub fn greet() {
  unsafe { alert("Hello, World!") };
}

#[wasm_bindgen]
pub fn greet2() {
  unsafe { console::log_1(&"Hello, World!".into()) };
}

#[wasm_bindgen]
extern "C" {
  fn alert(s: &str);
}
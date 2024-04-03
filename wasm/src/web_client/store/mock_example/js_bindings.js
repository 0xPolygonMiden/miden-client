// Example
#[wasm_bindgen(module = "/js/db/greet.js")]
extern "C" {
    #[wasm_bindgen(js_name = insertGreeting)]
    fn insert_greeting(greeting: String) -> js_sys::Promise;
}
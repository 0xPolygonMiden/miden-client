use wasm_bindgen::prelude::*;

use super::WebClient;

#[wasm_bindgen]
impl WebClient {
    pub async fn sync_state(
        &mut self
    ) -> () {
        if let Some(ref mut client) = self.get_mut_inner() {
            let message = client.sync_state().await;
            let js_value_message = JsValue::from_str(&message);
            
            // Print the message to the Chrome console
            web_sys::console::log_1(&js_value_message);
        } else {
            web_sys::console::error_1(&"Client not initialized".into());
        }
    }
}
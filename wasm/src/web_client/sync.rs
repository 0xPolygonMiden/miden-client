use wasm_bindgen::prelude::*;

use super::WebClient;

#[wasm_bindgen]
impl WebClient {
    pub async fn sync_state(
        &mut self
    ) -> Result<JsValue, JsValue> {
        if let Some(ref mut client) = self.get_mut_inner() {
            let block_num = client.sync_state().await.unwrap();

            Ok(JsValue::from_f64(block_num as f64))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}
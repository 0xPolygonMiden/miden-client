use wasm_bindgen::prelude::*;

use super::WebClient;

#[wasm_bindgen]
impl WebClient {
    pub async fn sync_state(
        &mut self
    ) -> Result<JsValue, JsValue> {
        if let Some(ref mut client) = self.get_mut_inner() {
            let block_num = client.sync_state().await.unwrap();

            let message = format!("State synced to block {}", block_num);
            Ok(JsValue::from_str(&message))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}
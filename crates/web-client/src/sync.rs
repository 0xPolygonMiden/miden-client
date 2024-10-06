use wasm_bindgen::prelude::*;

use crate::WebClient;

#[wasm_bindgen]
impl WebClient {
    pub async fn sync_state(&mut self) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let sync_summary = client.sync_state().await.unwrap();

            Ok(JsValue::from_f64(sync_summary.block_num as f64))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

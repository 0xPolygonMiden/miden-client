use wasm_bindgen::prelude::*;

use crate::WebClient;

#[wasm_bindgen]
impl WebClient {
    pub async fn sync_state(&mut self, update_ignored: bool) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let mut sync_summary = client.sync_state().await.unwrap();
            if update_ignored {
                sync_summary.combine_with(&client.update_ignored_notes().await.unwrap());
            }

            Ok(JsValue::from_f64(sync_summary.block_num as f64))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

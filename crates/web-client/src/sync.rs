use wasm_bindgen::prelude::*;

use crate::{WebClient, models::sync_summary::SyncSummary};

#[wasm_bindgen]
impl WebClient {
    #[wasm_bindgen(js_name = "syncState")]
    pub async fn sync_state(&mut self) -> Result<SyncSummary, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let sync_summary = client
                .sync_state()
                .await
                .map_err(|err| JsValue::from_str(&format!("Failed to sync state: {err}")))?;

            Ok(sync_summary.into())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

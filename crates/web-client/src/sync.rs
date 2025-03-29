use wasm_bindgen::prelude::*;

use crate::{WebClient, js_error_with_context, models::sync_summary::SyncSummary};

#[wasm_bindgen]
impl WebClient {
    #[wasm_bindgen(js_name = "syncState")]
    pub async fn sync_state(&mut self) -> Result<SyncSummary, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let sync_summary = client
                .sync_state()
                .await
                .map_err(|err| js_error_with_context(err, "failed to sync state"))?;

            Ok(sync_summary.into())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

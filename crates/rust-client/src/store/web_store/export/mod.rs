use crate::store::StoreError;

use super::WebStore;

mod js_bindings;
use js_bindings::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

impl WebStore {
    pub async fn export_store(&self) -> Result<JsValue, StoreError> {
        let promise = idxdb_export_store();
        let js_value = JsFuture::from(promise).await.map_err(|err| {
            StoreError::DatabaseError(format!("Failed to export store: {:?}", err))
        })?;
        Ok(js_value)
    }
}

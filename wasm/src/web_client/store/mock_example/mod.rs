use wasm_bindgen_futures::*;

use super::WebStore;
mod js_bindings;

impl WebStore {
    pub(crate) async fn insert_string(
        &mut self, 
        data: String
    ) -> Result<(), ()> {
        let result = JsFuture::from(js_bindings::insert_greeting(data)).await;
        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}
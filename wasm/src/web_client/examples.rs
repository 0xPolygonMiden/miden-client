use super::WebClient;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl WebClient {
    pub async fn test_store_and_rpc(&mut self) -> Result<JsValue, JsValue> {
        if let Some(ref mut client) = self.get_mut_inner() {
            let _ = client.store.insert_string("Test string".to_string()).await
                .map(|_| JsValue::from_str("Test string inserted successfully"))
                .map_err(|_| JsValue::from_str("Failed to insert test string"));

            client.rpc_api.test_rpc().await // This is the new line
                .map(|_| JsValue::from_str("RPC call successful"))
                .map_err(|_| JsValue::from_str("RPC call failed"))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}
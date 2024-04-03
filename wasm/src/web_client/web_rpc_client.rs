use crate::native_code::rpc::NodeRpcClient;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

use async_trait::async_trait;

#[wasm_bindgen(module = "/js/web-rpc-client.js")]
extern "C" {
    #[wasm_bindgen(js_name = testRpc)]
    fn test_rpc(endpoint: String) -> js_sys::Promise;
}

pub struct WebRpcClient {
    endpoint: String
}

impl WebRpcClient {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string()
        }
    }
}

#[async_trait(?Send)]
impl NodeRpcClient for WebRpcClient {
    async fn test_rpc(&mut self) -> Result<(), JsValue> {
        // Now correctly handling the Promise returned by test_rpc
        let promise = test_rpc("https://www.google.com".to_string());
        let result = JsFuture::from(promise).await;
        
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
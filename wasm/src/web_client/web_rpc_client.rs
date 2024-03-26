use crate::native_code::rpc::NodeRpcClient;

use async_trait::async_trait;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/js/web-rpc-client.js")]
extern "C" {
    #[wasm_bindgen(js_name = testRpc)]
    async fn test_rpc(endpoint: String) -> JsValue; // Directly return JsValue
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
    async fn test_rpc(&mut self) -> Result<(), ()> {
        let result = test_rpc(self.endpoint.clone()).await; // This now directly returns a JsValue
        match result {
            _ => Ok(()), // Treat any result as success; adjust as needed for actual error handling
        }
    }
}
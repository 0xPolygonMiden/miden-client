use wasm_bindgen::prelude::*;

use async_trait::async_trait;

#[async_trait(?Send)]
pub trait NodeRpcClient {
    // Test RPC method to be implemented by the client
    async fn test_rpc(&mut self) -> Result<(), JsValue>; 
}
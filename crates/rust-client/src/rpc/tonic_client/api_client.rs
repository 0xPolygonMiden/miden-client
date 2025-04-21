#[cfg(all(not(target_arch = "wasm32"), feature = "web-tonic"))]
compile_error!("The `web-tonic` feature is only supported when targeting wasm32.");

#[cfg(feature = "web-tonic")]
pub(crate) mod api_client_wrapper {
    use alloc::string::String;

    use crate::rpc::RpcError;

    pub type ApiClient =
        crate::rpc::generated::rpc::api_client::ApiClient<tonic_web_wasm_client::Client>;

    impl ApiClient {
        #[allow(clippy::unused_async)]
        pub async fn new_client(endpoint: String, _timeout_ms: u64) -> Result<ApiClient, RpcError> {
            let wasm_client = tonic_web_wasm_client::Client::new(endpoint);
            Ok(ApiClient::new(wasm_client))
        }
    }
}

#[cfg(feature = "tonic")]
pub(crate) mod api_client_wrapper {
    use alloc::{boxed::Box, string::String};
    use core::time::Duration;

    use crate::rpc::RpcError;

    pub type ApiClient =
        crate::rpc::generated::rpc::api_client::ApiClient<tonic::transport::Channel>;

    impl ApiClient {
        pub async fn new_client(endpoint: String, timeout_ms: u64) -> Result<ApiClient, RpcError> {
            let endpoint = tonic::transport::Endpoint::try_from(endpoint)
                .map_err(|err| RpcError::ConnectionError(Box::new(err)))?
                .timeout(Duration::from_millis(timeout_ms));

            ApiClient::connect(endpoint)
                .await
                .map_err(|err| RpcError::ConnectionError(Box::new(err)))
        }
    }
}

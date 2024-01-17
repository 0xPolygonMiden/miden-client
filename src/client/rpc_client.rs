// RPC Client
// ================================================================================================
//
use crate::errors::ClientError;
use crate::errors::RpcApiError;
use miden_node_proto::{
    requests::{SubmitProvenTransactionRequest, SyncStateRequest},
    responses::{SubmitProvenTransactionResponse, SyncStateResponse},
    rpc::api_client::ApiClient,
};
use tonic::transport::Channel;

// CONSTANTS
// ================================================================================================

pub enum RpcClientError {
    SynStateError(RpcApiError),
    SubmitProvenTxError(RpcApiError),
}

/// Wrapper for ApiClient which defers establishing a connection with a node until necessary
pub(crate) struct RpcClient {
    rpc_api: Option<ApiClient<Channel>>,
    endpoint: String,
}

impl RpcClient {
    pub fn new(config_endpoint: String) -> RpcClient {
        RpcClient {
            rpc_api: None,
            endpoint: config_endpoint,
        }
    }

    /// Executes the specified sync state request and returns the response.
    pub async fn sync_state(
        &mut self,
        request: impl tonic::IntoRequest<SyncStateRequest>,
    ) -> Result<tonic::Response<SyncStateResponse>, ClientError> {
        let rpc_api = self.rpc_api().await?;
        rpc_api
            .sync_state(request)
            .await
            .map_err(|err| ClientError::RpcApiError(RpcApiError::RequestError(err)))
    }

    pub async fn submit_proven_transaction(
        &mut self,
        request: impl tonic::IntoRequest<SubmitProvenTransactionRequest>,
    ) -> Result<tonic::Response<SubmitProvenTransactionResponse>, ClientError> {
        let rpc_api = self.rpc_api().await?;
        rpc_api
            .submit_proven_transaction(request)
            .await
            .map_err(|err| ClientError::RpcApiError(RpcApiError::RequestError(err)))
    }

    /// Takes care of establishing the rpc connection if not connected yet and returns a reference
    /// to the inner ApiClient
    async fn rpc_api(&mut self) -> Result<&mut ApiClient<Channel>, ClientError> {
        if self.rpc_api.is_some() {
            Ok(self.rpc_api.as_mut().unwrap())
        } else {
            let rpc_api = ApiClient::connect(self.endpoint.clone())
                .await
                .map_err(|err| ClientError::RpcApiError(RpcApiError::ConnectionError(err)))?;
            Ok(self.rpc_api.insert(rpc_api))
        }
    }
}

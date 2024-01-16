// MIDEN CLIENT
// ================================================================================================

#[cfg(not(any(test, feature = "mock")))]
use crate::errors::RpcApiError;
use crate::{config::ClientConfig, errors::ClientError, store::Store};
#[cfg(not(any(test, feature = "mock")))]
use miden_node_proto::{
    requests::{SubmitProvenTransactionRequest, SyncStateRequest},
    responses::{SubmitProvenTransactionResponse, SyncStateResponse},
};

use miden_tx::TransactionExecutor;

pub mod accounts;
pub mod chain_data;
pub mod notes;
pub mod sync_state;
pub mod transactions;

// CONSTANTS
// ================================================================================================

#[cfg(not(any(test, feature = "mock")))]
struct LazyRpcClient(
    Option<miden_node_proto::rpc::api_client::ApiClient<tonic::transport::Channel>>,
    String,
);

#[cfg(not(any(test, feature = "mock")))]
impl LazyRpcClient {
    pub fn new(config_endpoint: String) -> LazyRpcClient {
        LazyRpcClient(None, config_endpoint)
    }

    /// Executes the specified sync state request and returns the response.
    pub async fn sync_state(
        &mut self,
        request: impl tonic::IntoRequest<SyncStateRequest>,
    ) -> std::result::Result<tonic::Response<SyncStateResponse>, ClientError> {
        let rpc_api = self.rpc_api().await?;
        rpc_api
            .sync_state(request)
            .await
            .map_err(|err| ClientError::RpcApiError(RpcApiError::RequestError(err)))
    }

    pub async fn submit_proven_transaction(
        &mut self,
        request: impl tonic::IntoRequest<SubmitProvenTransactionRequest>,
    ) -> std::result::Result<tonic::Response<SubmitProvenTransactionResponse>, ClientError> {
        let rpc_api = self.rpc_api().await?;
        rpc_api
            .submit_proven_transaction(request)
            .await
            .map_err(|err| ClientError::RpcApiError(RpcApiError::RequestError(err)))
    }

    /// Takes care of establishing the rpc connection if not connected yet and returns a reference
    /// to the inner ApiClient
    async fn rpc_api(
        &mut self,
    ) -> Result<
        &mut miden_node_proto::rpc::api_client::ApiClient<tonic::transport::Channel>,
        ClientError,
    > {
        use miden_node_proto::rpc::api_client::ApiClient;

        if self.0.is_some() {
            Ok(self.0.as_mut().unwrap())
        } else {
            let rpc_api = ApiClient::connect(self.1.clone())
                .await
                .map_err(|err| ClientError::RpcApiError(RpcApiError::ConnectionError(err)))?;
            Ok(self.0.insert(rpc_api))
        }
    }
}

/// A light client for connecting to the Miden rollup network.
///
/// Miden client is responsible for managing a set of accounts. Specifically, the client:
/// - Keeps track of the current and historical states of a set of accounts and related objects
///   such as notes and transactions.
/// - Connects to one or more Miden nodes to periodically sync with the current state of the
///   network.
/// - Executes, proves, and submits transactions to the network as directed by the user.
#[cfg(not(any(test, feature = "mock")))]
pub struct Client {
    /// Local database containing information about the accounts managed by this client.
    store: Store,
    rpc_api: LazyRpcClient,
    tx_executor: TransactionExecutor<crate::store::data_store::SqliteDataStore>,
}

#[cfg(not(any(test, feature = "mock")))]
impl Client {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client] instantiated with the specified configuration options.
    ///
    /// # Errors
    /// Returns an error if the client could not be instantiated.
    pub async fn new(config: ClientConfig) -> Result<Self, ClientError> {
        use crate::store::data_store::SqliteDataStore;

        Ok(Self {
            store: Store::new((&config).into())?,
            rpc_api: LazyRpcClient::new(config.node_endpoint.to_string()),
            tx_executor: TransactionExecutor::new(SqliteDataStore::new(Store::new(
                (&config).into(),
            )?)),
        })
    }
}

// TESTING
// ================================================================================================

#[cfg(any(test, feature = "mock"))]
pub struct Client {
    pub(crate) store: Store,
    pub(crate) rpc_api: crate::mock::MockRpcApi,
    pub(crate) tx_executor:
        TransactionExecutor<crate::store::mock_executor_data_store::MockDataStore>,
}

#[cfg(any(test, feature = "mock"))]
impl Client {
    pub async fn new(config: ClientConfig) -> Result<Self, ClientError> {
        use crate::store::mock_executor_data_store::MockDataStore;

        Ok(Self {
            store: Store::new((&config).into())?,
            rpc_api: Default::default(),
            tx_executor: TransactionExecutor::new(MockDataStore::new()),
        })
    }
}

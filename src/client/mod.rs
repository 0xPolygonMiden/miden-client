// MIDEN CLIENT
// ================================================================================================

use crate::{
    config::ClientConfig,
    errors::ClientError,
    store::{mock_executor_data_store::MockDataStore, Store},
};

#[cfg(not(any(test, feature = "testing")))]
use crate::errors::RpcApiError;

use miden_tx::TransactionExecutor;

#[cfg(any(test, feature = "testing"))]
use crate::mock::MockRpcApi;

pub mod accounts;
pub mod notes;
pub mod sync_state;
pub mod transactions;

// CONSTANTS
// ================================================================================================

/// The number of bits to shift identifiers for in use of filters.
pub const FILTER_ID_SHIFT: u8 = 48;

/// A light client for connecting to the Miden rollup network.
///
/// Miden client is responsible for managing a set of accounts. Specifically, the client:
/// - Keeps track of the current and historical states of a set of accounts and related objects
///   such as notes and transactions.
/// - Connects to one or more Miden nodes to periodically sync with the current state of the
///   network.
/// - Executes, proves, and submits transactions to the network as directed by the user.
pub struct Client {
    /// Local database containing information about the accounts managed by this client.
    pub(crate) store: Store,
    #[cfg(not(any(test, feature = "testing")))]
    /// Api client for interacting with the Miden node.
    rpc_api: miden_node_proto::rpc::api_client::ApiClient<tonic::transport::Channel>,
    #[cfg(any(test, feature = "testing"))]
    pub rpc_api: MockRpcApi,
    pub(crate) tx_executor: TransactionExecutor<MockDataStore>,
}

impl Client {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client] instantiated with the specified configuration options.
    ///
    /// # Errors
    /// Returns an error if the client could not be instantiated.
    pub async fn new(config: ClientConfig) -> Result<Self, ClientError> {
        Ok(Self {
            store: Store::new((&config).into())?,
            #[cfg(not(any(test, feature = "testing")))]
            rpc_api: miden_node_proto::rpc::api_client::ApiClient::connect(
                config.node_endpoint.to_string(),
            )
            .await
            .map_err(|err| ClientError::RpcApiError(RpcApiError::ConnectionError(err)))?,
            #[cfg(any(test, feature = "testing"))]
            rpc_api: Default::default(),
            tx_executor: TransactionExecutor::new(MockDataStore::new()),
        })
    }
}

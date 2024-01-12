// MIDEN CLIENT
// ================================================================================================

use crate::{config::ClientConfig, errors::ClientError, store::Store};

use miden_tx::TransactionExecutor;

pub mod accounts;
pub mod chain_data;
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
    #[cfg(any(test, feature = "mock"))]
    pub rpc_api: crate::mock::MockRpcApi,
    #[cfg(any(test, feature = "mock"))]
    pub(crate) tx_executor:
    TransactionExecutor<crate::store::mock_executor_data_store::MockDataStore>,
    #[cfg(not(any(test, feature = "mock")))]
    pub rpc_api: miden_node_proto::rpc::api_client::ApiClient<tonic::transport::Channel>,
    #[cfg(not(any(test, feature = "mock")))]
    pub tx_executor: TransactionExecutor<crate::store::data_store::SqliteDataStore>,
}

impl Client {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client] instantiated with the specified configuration options.
    ///
    /// # Errors
    /// Returns an error if the client could not be instantiated.
    pub async fn new(config: ClientConfig) -> Result<Self, ClientError> {
        #[cfg(not(any(test, feature = "mock")))]
        return Ok(Self {
            store: Store::new((&config).into())?,
            rpc_api: miden_node_proto::rpc::api_client::ApiClient::connect(
                config.node_endpoint.to_string(),
            )
            .await
            .map_err(|err| {
                ClientError::RpcApiError(crate::errors::RpcApiError::ConnectionError(err))
            })?,
            tx_executor: TransactionExecutor::new(crate::store::data_store::SqliteDataStore::new(
                Store::new((&config).into())?,
            )),
        });

        #[cfg(any(test, feature = "mock"))]
        return Ok(Self {
            store: Store::new((&config).into())?,
            rpc_api: Default::default(),
            tx_executor: TransactionExecutor::new(
                crate::store::mock_executor_data_store::MockDataStore::new(),
            ),
        });
    }
}

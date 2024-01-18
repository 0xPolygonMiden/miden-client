// MIDEN CLIENT
// ================================================================================================

use crate::{config::ClientConfig, errors::ClientError, store::Store};
use core::fmt;
use miden_tx::TransactionExecutor;

pub mod accounts;
pub mod chain_data;
pub mod notes;
#[cfg(not(any(test, feature = "mock")))]
pub mod rpc_client;
pub mod sync_state;
pub mod transactions;

// CONSTANTS
// ================================================================================================

#[derive(Debug)]
pub enum RpcApiEndpoint {
    SyncState,
    SubmitProvenTx,
}

impl fmt::Display for RpcApiEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcApiEndpoint::SyncState => write!(f, "sync_state"),
            RpcApiEndpoint::SubmitProvenTx => write!(f, "submit_proven_transaction"),
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
    rpc_api: rpc_client::RpcClient,
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
            rpc_api: RpcClient::new(config.node_endpoint.to_string()),
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

#[cfg(not(any(test, feature = "mock")))]
use crate::store::data_store::SqliteDataStore;
use crate::{config::ClientConfig, errors::ClientError, store::Store};
use miden_tx::TransactionExecutor;
pub use rpc_client::RpcApiEndpoint;

pub mod accounts;
mod chain_data;
mod notes;
mod rpc_client;
pub(crate) mod sync_state;
pub mod transactions;

// MIDEN CLIENT
// ================================================================================================

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
    tx_executor: TransactionExecutor<SqliteDataStore>,
}

#[cfg(not(any(test, feature = "mock")))]
impl Client {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client] instantiated with the specified configuration options.
    ///
    /// # Errors
    /// Returns an error if the client could not be instantiated.
    pub fn new(config: ClientConfig) -> Result<Self, ClientError> {
        Ok(Self {
            store: Store::new((&config).into())?,
            rpc_api: rpc_client::RpcClient::new(config.rpc.endpoint.to_string()),
            tx_executor: TransactionExecutor::new(SqliteDataStore::new(Store::new(
                (&config).into(),
            )?)),
        })
    }
}

// TESTING
// ================================================================================================

#[cfg(any(test, feature = "mock"))]
pub use mock::Client;

#[cfg(any(test, feature = "mock"))]
mod mock {
    use super::{ClientConfig, ClientError, Store, TransactionExecutor};
    use crate::{mock::MockRpcApi, store::mock_executor_data_store::MockDataStore};

    pub struct Client {
        pub(crate) store: Store,
        pub(crate) rpc_api: MockRpcApi,
        pub(crate) tx_executor: TransactionExecutor<MockDataStore>,
    }

    #[cfg(any(test, feature = "mock"))]
    impl Client {
        pub fn new(config: ClientConfig) -> Result<Self, ClientError> {
            Ok(Self {
                store: Store::new((&config).into())?,
                rpc_api: Default::default(),
                tx_executor: TransactionExecutor::new(MockDataStore::new()),
            })
        }
    }
}

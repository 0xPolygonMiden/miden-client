use crate::{config::ClientConfig, errors::ClientError, store::Store};
use miden_tx::{DataStore, TransactionExecutor};

pub mod rpc;
use rpc::NodeRpcClient;

pub mod accounts;
mod chain_data;
mod notes;
pub(crate) mod sync;
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
pub struct Client<N: NodeRpcClient, D: DataStore> {
    /// Local database containing information about the accounts managed by this client.
    store: Store,
    rpc_api: N,
    tx_executor: TransactionExecutor<D>,
}

impl<N: NodeRpcClient, D: DataStore> Client<N, D> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client] instantiated with the specified configuration options.
    ///
    /// # Errors
    /// Returns an error if the client could not be instantiated.
    pub fn new(config: ClientConfig, api: N, data_store: D) -> Result<Self, ClientError> {
        Ok(Self {
            store: Store::new((&config).into())?,
            rpc_api: api,
            tx_executor: TransactionExecutor::new(data_store),
        })
    }

    #[cfg(any(test, feature = "mock"))]
    pub fn rpc_api(&mut self) -> &mut N {
        &mut self.rpc_api
    }

    #[cfg(any(test, feature = "mock"))]
    pub fn set_tx_executor(&mut self, tx_executor: TransactionExecutor<D>) {
        self.tx_executor = tx_executor;
    }

    #[cfg(any(test, feature = "mock"))]
    pub fn store(&mut self) -> &mut Store {
        &mut self.store
    }
}

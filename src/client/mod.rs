use crate::{errors::ClientError, store::Store};
use miden_tx::TransactionExecutor;

pub mod rpc;
use rpc::NodeRpcClient;

pub mod accounts;
mod chain_data;
mod notes;
pub(crate) mod sync;
pub mod transactions;

#[cfg(any(test, feature = "mock"))]
use crate::mock::MockDataStore;
#[cfg(not(any(test, feature = "mock")))]
use crate::store::data_store::ClientDataStore;
#[cfg(not(any(test, feature = "mock")))]
use std::{cell::RefCell, rc::Rc};

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
pub struct Client<N: NodeRpcClient, S: Store> {
    /// Local database containing information about the accounts managed by this client.
    store: S,
    rpc_api: N,
    #[cfg(not(any(test, feature = "mock")))]
    tx_executor: TransactionExecutor<ClientDataStore<S>>,
    #[cfg(any(test, feature = "mock"))]
    tx_executor: TransactionExecutor<MockDataStore>,
}

impl<N: NodeRpcClient, S: Store> Client<N, S> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client] instantiated with the specified configuration options.
    ///
    /// # Errors
    /// Returns an error if the client could not be instantiated.
    #[cfg(not(any(test, feature = "mock")))]
    pub fn new(api: N, store: S, data_store_store: S) -> Result<Self, ClientError> {
        Ok(Self {
            store,
            rpc_api: api,
            tx_executor: TransactionExecutor::new(ClientDataStore::new(data_store_store)),
        })
    }

    #[cfg(any(test, feature = "mock"))]
    pub fn new(api: N, store: S, data_store: MockDataStore) -> Result<Self, ClientError> {
        Ok(Self {
            store,
            rpc_api: api,
            tx_executor: TransactionExecutor::new(data_store),
        })
    }

    #[cfg(any(test, feature = "mock"))]
    pub fn rpc_api(&mut self) -> &mut N {
        &mut self.rpc_api
    }

    #[cfg(any(test, feature = "mock"))]
    pub fn set_tx_executor(&mut self, tx_executor: TransactionExecutor<MockDataStore>) {
        self.tx_executor = tx_executor;
    }

    #[cfg(any(test, feature = "mock"))]
    pub fn store(&mut self) -> &mut S {
        &mut self.store
    }
}

#[cfg(not(any(test, feature = "mock")))]
pub fn shared_store_client<N: NodeRpcClient, S: Store>(
    api: N,
    store: S,
) -> Result<Client<N, Rc<RefCell<S>>>, ClientError> {
    let store = Rc::new(RefCell::new(store));

    Client::new(api, store.clone(), store)
}

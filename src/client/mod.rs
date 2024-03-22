use miden_tx::TransactionExecutor;

use crate::{errors::ClientError, store::Store};

pub mod rpc;
use rpc::NodeRpcClient;

pub mod accounts;
#[cfg(test)]
mod chain_data;
mod note_consumption_checker;
mod notes;
pub(crate) mod sync;
pub mod transactions;
pub(crate) use note_consumption_checker::filter_created_notes_to_track;

use crate::store::data_store::ClientDataStore;

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
    /// The client's store, which provides a way to write and read entities to provide persistence.
    store: S,
    /// An instance of [NodeRpcClient] which provides a way for the client to connect to the Miden node.
    rpc_api: N,
    tx_executor: TransactionExecutor<ClientDataStore<S>>,
}

impl<N: NodeRpcClient, S: Store> Client<N, S> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client].
    ///
    /// ## Arguments
    ///
    /// - `api`: An instance of [NodeRpcClient] which provides a way for the client to connect to the Miden node.
    /// - `store`: An instance of [Store], which provides a way to write and read entities to provide persistence.
    /// - `executor_store`: An instance of [Store] that provides a way for [TransactionExecutor] to
    /// retrieve relevant inputs at the moment of transaction execution. It should be the same
    /// store as the one for `store`, but it doesn't have to be the **same instance**
    ///
    /// # Errors
    ///
    /// Returns an error if the client could not be instantiated.
    pub fn new(
        api: N,
        store: S,
        executor_store: S,
    ) -> Result<Self, ClientError> {
        Ok(Self {
            store,
            rpc_api: api,
            tx_executor: TransactionExecutor::new(ClientDataStore::new(executor_store)),
        })
    }

    #[cfg(any(test, feature = "test_utils"))]
    pub fn rpc_api(&mut self) -> &mut N {
        &mut self.rpc_api
    }

    #[cfg(any(test, feature = "test_utils"))]
    pub fn store(&mut self) -> &mut S {
        &mut self.store
    }
}

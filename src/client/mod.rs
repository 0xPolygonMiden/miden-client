use crate::{
    config::ClientConfig,
    errors::{ClientError, NodeApiError},
    store::Store,
};
use async_trait::async_trait;
use miden_tx::{DataStore, TransactionExecutor};
use objects::{accounts::AccountId, transaction::ProvenTransaction, BlockHeader};

pub use rpc_client::RpcApiEndpoint;
use rpc_client::StateSyncInfo;

pub mod accounts;
mod chain_data;
mod notes;
pub mod rpc_client;
pub(crate) mod sync;
pub mod transactions;

// NODE API TRAIT
// ================================================================================================

#[async_trait]
pub trait NodeApi {
    fn new(config_endpoint: &str) -> Self;
    async fn submit_proven_transaction(
        &mut self,
        proven_transaction: ProvenTransaction,
    ) -> Result<(), NodeApiError>;
    async fn get_block_header_by_number(
        &mut self,
        block_number: Option<u32>,
    ) -> Result<BlockHeader, NodeApiError>;
    async fn sync_state(
        &mut self,
        block_num: u32,
        account_ids: &[AccountId],
        note_tags: &[u16],
        nullifiers_tags: &[u16],
    ) -> Result<StateSyncInfo, NodeApiError>;
}

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
pub struct Client<N: NodeApi, D: DataStore> {
    /// Local database containing information about the accounts managed by this client.
    store: Store,
    rpc_api: N,
    tx_executor: TransactionExecutor<D>,
}

impl<N: NodeApi, D: DataStore> Client<N, D> {
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

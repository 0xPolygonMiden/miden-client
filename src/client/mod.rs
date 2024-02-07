#[cfg(not(any(test, feature = "mock")))]
use crate::store::data_store::SqliteDataStore;
use crate::{
    config::ClientConfig,
    errors::{ClientError, NodeApiError},
    store::Store,
};
use miden_tx::TransactionExecutor;
use objects::{accounts::AccountId, transaction::ProvenTransaction, BlockHeader};
pub use rpc_client::RpcApiEndpoint;

pub mod accounts;
mod chain_data;
mod notes;
pub mod rpc_client;
pub(crate) mod sync;
pub mod transactions;

// NODE API TRAIT
// ================================================================================================

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
#[cfg(not(any(test, feature = "mock")))]
pub struct Client<N: NodeApi> {
    /// Local database containing information about the accounts managed by this client.
    store: Store,
    rpc_api: N,
    tx_executor: TransactionExecutor<SqliteDataStore>,
}

#[cfg(not(any(test, feature = "mock")))]
impl<N: NodeApi> Client<N> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client] instantiated with the specified configuration options.
    ///
    /// # Errors
    /// Returns an error if the client could not be instantiated.
    pub fn new(config: ClientConfig, api: N) -> Result<Self, ClientError> {
        Ok(Self {
            store: Store::new((&config).into())?,
            rpc_api: api,
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

use self::rpc_client::StateSyncInfo;

#[cfg(any(test, feature = "mock"))]
mod mock {
    use super::{ClientConfig, ClientError, NodeApi, Store, TransactionExecutor};
    use crate::store::mock_executor_data_store::MockDataStore;

    pub struct Client<N: NodeApi> {
        pub(crate) store: Store,
        pub(crate) rpc_api: N,
        pub(crate) tx_executor: TransactionExecutor<MockDataStore>,
    }

    #[cfg(any(test, feature = "mock"))]
    impl<N: NodeApi> Client<N> {
        pub fn new(config: ClientConfig, api: N) -> Result<Self, ClientError> {
            Ok(Self {
                store: Store::new((&config).into())?,
                rpc_api: api,
                tx_executor: TransactionExecutor::new(MockDataStore::new()),
            })
        }
    }
}

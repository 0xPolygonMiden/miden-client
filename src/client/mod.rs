use alloc::rc::Rc;

use miden_objects::{
    crypto::rand::{FeltRng, RpoRandomCoin},
    Felt,
};
use miden_tx::{TransactionAuthenticator, TransactionExecutor};
use rand::Rng;
use tracing::info;

use crate::store::{data_store::ClientDataStore, Store};

pub mod rpc;
use rpc::NodeRpcClient;

pub mod accounts;
#[cfg(test)]
mod chain_data;
mod note_screener;
mod notes;
pub mod store_authenticator;
pub mod sync;
pub mod transactions;
pub(crate) use note_screener::NoteScreener;
pub use note_screener::{NoteConsumability, NoteRelevance};

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
pub struct Client<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> {
    /// The client's store, which provides a way to write and read entities to provide persistence.
    store: Rc<S>,
    /// An instance of [FeltRng] which provides randomness tools for generating new keys,
    /// serial numbers, etc.
    rng: R,
    /// An instance of [NodeRpcClient] which provides a way for the client to connect to the
    /// Miden node.
    rpc_api: N,
    tx_executor: TransactionExecutor<ClientDataStore<S>, A>,
}

impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> Client<N, R, S, A> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client].
    ///
    /// ## Arguments
    ///
    /// - `api`: An instance of [NodeRpcClient] which provides a way for the client to connect
    ///   to the Miden node.
    /// - `store`: An instance of [Store], which provides a way to write and read entities to
    ///   provide persistence.
    /// - `executor_store`: An instance of [Store] that provides a way for [TransactionExecutor] to
    ///   retrieve relevant inputs at the moment of transaction execution. It should be the same
    ///   store as the one for `store`, but it doesn't have to be the **same instance**.
    /// - `authenticator`: Defines the transaction authenticator that will be used by the
    ///   transaction executor whenever a signature is requested from within the VM.
    /// - `in_debug_mode`: Instantiates the transaction executor (and in turn, its compiler)
    ///   in debug mode, which will enable debug logs for scripts compiled with this mode for
    ///   easier MASM debugging.
    ///
    /// # Errors
    ///
    /// Returns an error if the client could not be instantiated.
    pub fn new(api: N, rng: R, store: Rc<S>, authenticator: A, in_debug_mode: bool) -> Self {
        if in_debug_mode {
            info!("Creating the Client in debug mode.");
        }

        let data_store = ClientDataStore::new(store.clone());
        let authenticator = Some(Rc::new(authenticator));
        let tx_executor = TransactionExecutor::new(data_store, authenticator);

        Self { store, rng, rpc_api: api, tx_executor }
    }

    #[cfg(any(test, feature = "test_utils"))]
    pub fn rpc_api(&mut self) -> &mut N {
        &mut self.rpc_api
    }

    #[cfg(any(test, feature = "test_utils"))]
    pub fn store(&mut self) -> &S {
        &self.store
    }
}

// HELPERS
// --------------------------------------------------------------------------------------------

/// Gets [RpoRandomCoin] from the client
pub fn get_random_coin() -> RpoRandomCoin {
    // TODO: Initialize coin status once along with the client and persist status for retrieval
    let mut rng = rand::thread_rng();
    let coin_seed: [u64; 4] = rng.gen();

    RpoRandomCoin::new(coin_seed.map(Felt::new))
}

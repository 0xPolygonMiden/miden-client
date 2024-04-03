use miden_objects::{
    crypto::rand::{FeltRng, RpoRandomCoin},
    Felt,
};
use miden_tx::TransactionExecutor;
use rand::Rng;

use crate::{errors::ClientError, store::Store};

pub mod rpc;
use rpc::NodeRpcClient;

pub mod accounts;
#[cfg(test)]
mod chain_data;
mod note_screener;
mod notes;
pub(crate) mod sync;
pub mod transactions;
pub(crate) use note_screener::NoteScreener;

use crate::store::data_store::ClientDataStore;

// CLIENT RNG
// ================================================================================================
pub trait ClientRng: FeltRng {
    fn get_random_seed(&mut self) -> [u8; 32] {
        let mut seed = [0; 32];
        let word = self.draw_word();

        seed[0..8].copy_from_slice(&word[0].inner().to_le_bytes());
        seed[8..16].copy_from_slice(&word[1].inner().to_le_bytes());
        seed[16..24].copy_from_slice(&word[2].inner().to_le_bytes());
        seed[24..32].copy_from_slice(&word[3].inner().to_le_bytes());

        seed
    }
}

impl<T: FeltRng> ClientRng for T {}

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
pub struct Client<N: NodeRpcClient, R: ClientRng, S: Store> {
    /// The client's store, which provides a way to write and read entities to provide persistence.
    store: S,
    /// An instance of [ClientRng] which provides randomness tools for generating new keys,
    /// serial numbers, etc.
    rng: R,
    /// An instance of [NodeRpcClient] which provides a way for the client to connect to the
    /// Miden node.
    rpc_api: N,
    tx_executor: TransactionExecutor<ClientDataStore<S>>,
}

impl<N: NodeRpcClient, R: ClientRng, S: Store> Client<N, R, S> {
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
        rng: R,
        store: S,
        executor_store: S,
    ) -> Result<Self, ClientError> {
        Ok(Self {
            store,
            rng,
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

// HELPERS
// --------------------------------------------------------------------------------------------

/// Gets [RpoRandomCoin] from the client
pub fn get_random_coin() -> RpoRandomCoin {
    // TODO: Initialize coin status once along with the client and persist status for retrieval
    let mut rng = rand::thread_rng();
    let coin_seed: [u64; 4] = rng.gen();

    RpoRandomCoin::new(coin_seed.map(Felt::new))
}

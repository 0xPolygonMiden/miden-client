
use miden_objects::{
    crypto::rand::{FeltRng, RpoRandomCoin},
    Felt
};
use rand::{rngs::StdRng, Rng, SeedableRng};

pub mod accounts;
pub mod notes;
pub mod transactions;
pub mod sync;
pub mod chain_data;
pub mod utils;
pub mod errors;

pub mod store;
use store::Store;

pub mod rpc;
use rpc::NodeRpcClient;

// Hoping that eventually we can use the generic store type defined in client/mod.rs.
// For now, wanted to play around with creating a client with a WebStore implementation
// (instead of a SQLite implementation) and getting an underlying store method to execute
// in the browser.

// TODO: Remove pub from store field
// TODO: Add back generic type for NodeRpcClient and get example working in browser
// TODO: Add back generic type for DataStore and get example working in browser
pub struct Client<N: NodeRpcClient, R: FeltRng, S: Store> {
    pub store: S,
    pub rng: R,
    pub rpc_api: N,
    // pub tx_executor: TransactionExecutor<ClientDataStore<S>>
}

impl<N: NodeRpcClient, R: FeltRng, S: Store> Client<N, R, S> {
    pub fn new(
        api: N,
        rng: R,
        store: S,
        //executor_store: S
    ) -> Self {
        Self { 
            store: store,
            rng: rng,
            rpc_api: api,
            // tx_executor: TransactionExecutor::new(ClientDataStore::new(executor_store)) 
        }
    }
}

/// Gets [RpoRandomCoin] from the client
pub fn get_random_coin() -> RpoRandomCoin {
    // TODO: Initialize coin status once along with the client and persist status for retrieval
    let mut rng = StdRng::from_entropy();
    let coin_seed: [u64; 4] = rng.gen();

    RpoRandomCoin::new(coin_seed.map(Felt::new))
}
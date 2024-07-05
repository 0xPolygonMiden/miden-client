#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod accounts;
pub mod config;
mod errors;
pub mod notes;
pub mod rpc;
pub mod store;
mod store_authenticator;
pub mod sync;
pub mod transactions;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub mod tests;

// RE-EXPORTS
// ================================================================================================

pub mod assembly {
    pub use miden_objects::assembly::{AstSerdeOptions, ModuleAst, ProgramAst};
}

pub mod assets {
    pub use miden_objects::assets::{Asset, AssetVault, FungibleAsset, TokenSymbol};
}

pub mod auth {
    pub use miden_objects::accounts::AuthSecretKey;
    pub use miden_tx::auth::TransactionAuthenticator;

    pub use crate::store_authenticator::StoreAuthenticator;
}

pub mod blocks {
    pub use miden_objects::BlockHeader;
}

pub mod crypto {
    pub use miden_objects::{
        crypto::{
            merkle::{
                InOrderIndex, LeafIndex, MerklePath, MmrDelta, MmrPeaks, MmrProof, SmtLeaf,
                SmtProof,
            },
            rand::{FeltRng, RpoRandomCoin},
        },
        Digest,
    };
}

pub use errors::{ClientError, IdPrefixFetchError};
pub use miden_objects::{Felt, StarkField, Word, ONE, ZERO};

pub mod utils {
    pub use miden_tx::utils::{
        bytes_to_hex_string, ByteReader, ByteWriter, Deserializable, DeserializationError,
        Serializable,
    };
}

#[cfg(feature = "testing")]
pub mod testing {
    pub use miden_objects::accounts::account_id::testing::*;
}

use alloc::rc::Rc;
#[cfg(feature = "testing")]
use alloc::vec::Vec;

use miden_objects::crypto::rand::FeltRng;
use miden_tx::{auth::TransactionAuthenticator, TransactionExecutor};
use rpc::NodeRpcClient;
use store::{data_store::ClientDataStore, Store};
use tracing::info;

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
        let tx_executor =
            TransactionExecutor::new(data_store, authenticator).with_debug_mode(in_debug_mode);

        Self { store, rng, rpc_api: api, tx_executor }
    }

    // TEST HELPERS
    // --------------------------------------------------------------------------------------------

    #[cfg(any(test, feature = "testing"))]
    pub fn rpc_api(&mut self) -> &mut N {
        &mut self.rpc_api
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn store(&mut self) -> &S {
        &self.store
    }

    #[cfg(any(test, feature = "testing"))]
    #[winter_maybe_async::maybe_async]
    pub fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(miden_objects::BlockHeader, bool)>, crate::ClientError> {
        let result = winter_maybe_async::maybe_await!(self.store.get_block_headers(block_numbers))?;
        Ok(result)
    }
}

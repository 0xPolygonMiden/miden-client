#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod accounts;
pub mod config;
pub mod notes;
pub mod rpc;
pub mod store;
pub mod sync;
pub mod transactions;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub mod tests;

mod errors;
mod store_authenticator;

// RE-EXPORTS
// ================================================================================================

/// Provides types and utilities for working with assets within the Miden rollup network.
pub mod assets {
    pub use miden_objects::{
        accounts::delta::{
            AccountVaultDelta, FungibleAssetDelta, NonFungibleAssetDelta, NonFungibleDeltaAction,
        },
        assets::{Asset, AssetVault, FungibleAsset, NonFungibleAsset, TokenSymbol},
    };
}

/// Provides authentication-related types and functionalities for the Miden
/// rollup network.
pub mod auth {
    pub use miden_objects::accounts::AuthSecretKey;
    pub use miden_tx::auth::TransactionAuthenticator;

    pub use crate::store_authenticator::StoreAuthenticator;
}

/// Provides types for working with blocks within the Miden rollup network.
pub mod blocks {
    pub use miden_objects::BlockHeader;
}

/// Provides cryptographic types and utilities used within the Miden rollup
/// network. It re-exports commonly used types and random number generators like `FeltRng` from
/// the `miden_objects` crate.
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

use std::boxed::Box;

pub use errors::{ClientError, IdPrefixFetchError};
pub use miden_objects::{Felt, StarkField, Word, ONE, ZERO};

/// Provides various utilities that are commonly used throughout the Miden
/// client library.
pub mod utils {
    pub use miden_tx::utils::{
        bytes_to_hex_string, ByteReader, ByteWriter, Deserializable, DeserializationError,
        Serializable,
    };
}

/// Provides test utilities for working with accounts and account IDs
/// within the Miden rollup network. This module is only available when the `testing` feature is
/// enabled.
#[cfg(feature = "testing")]
pub mod testing {
    pub use miden_objects::{accounts::account_id::testing::*, testing::*};
}

use alloc::rc::Rc;

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
/// - Keeps track of the current and historical states of a set of accounts and related objects such
///   as notes and transactions.
/// - Connects to one or more Miden nodes to periodically sync with the current state of the
///   network.
/// - Executes, proves, and submits transactions to the network as directed by the user.
pub struct Client {
    /// The client's store, which provides a way to write and read entities to provide persistence.
    store: Rc<dyn Store>,
    /// An instance of [FeltRng] which provides randomness tools for generating new keys,
    /// serial numbers, etc.
    rng: Box<dyn FeltRng>,
    /// An instance of [NodeRpcClient] which provides a way for the client to connect to the
    /// Miden node.
    rpc_api: Box<dyn NodeRpcClient + Send>,
    tx_executor: TransactionExecutor<ClientDataStore, Box<dyn TransactionAuthenticator>>,
}

impl Client {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client].
    ///
    /// ## Arguments
    ///
    /// - `api`: An instance of [NodeRpcClient] which provides a way for the client to connect to
    ///   the Miden node.
    /// - `store`: An instance of [Store], which provides a way to write and read entities to
    ///   provide persistence.
    /// - `executor_store`: An instance of [Store] that provides a way for [TransactionExecutor] to
    ///   retrieve relevant inputs at the moment of transaction execution. It should be the same
    ///   store as the one for `store`, but it doesn't have to be the **same instance**.
    /// - `authenticator`: Defines the transaction authenticator that will be used by the
    ///   transaction executor whenever a signature is requested from within the VM.
    /// - `in_debug_mode`: Instantiates the transaction executor (and in turn, its compiler) in
    ///   debug mode, which will enable debug logs for scripts compiled with this mode for easier
    ///   MASM debugging.
    ///
    /// # Errors
    ///
    /// Returns an error if the client could not be instantiated.
    pub fn new(
        api: Box<dyn NodeRpcClient + Send>,
        rng: Box<dyn FeltRng>,
        store: Rc<dyn Store>,
        authenticator: Rc<dyn TransactionAuthenticator>,
        in_debug_mode: bool,
    ) -> Self {
        if in_debug_mode {
            info!("Creating the Client in debug mode.");
        }

        let data_store = ClientDataStore::new(store.clone());
        let authenticator = Some(authenticator);
        let tx_executor =
            TransactionExecutor::new(data_store, authenticator).with_debug_mode(in_debug_mode);

        Self { store, rng, rpc_api: api, tx_executor }
    }

    /// Returns a reference to the client's random number generator. This can be used to generate
    /// randomness for various purposes such as serial numbers, keys, etc.
    pub fn rng(&mut self) -> &mut dyn FeltRng {
        self.rng.as_mut()
    }

    // TEST HELPERS
    // --------------------------------------------------------------------------------------------

    #[cfg(any(test, feature = "testing"))]
    pub fn rpc_api(&mut self) -> &mut Box<dyn NodeRpcClient + Send> {
        &mut self.rpc_api
    }

    // TODO: the idxdb feature access here is temporary and should be removed in the future once
    // a good solution to the syncrhonous store access in the store authenticator is found.
    // https://github.com/0xPolygonMiden/miden-base/issues/705
    #[cfg(any(test, feature = "testing", feature = "idxdb"))]
    pub fn store(&mut self) -> &Rc<dyn Store> {
        &self.store
    }

    #[cfg(any(test, feature = "testing"))]
    #[winter_maybe_async::maybe_async]
    pub fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<alloc::vec::Vec<(miden_objects::BlockHeader, bool)>, crate::ClientError> {
        let result = winter_maybe_async::maybe_await!(self.store.get_block_headers(block_numbers))?;
        Ok(result)
    }
}

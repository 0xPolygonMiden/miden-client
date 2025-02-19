//! A no_std-compatible client library for interacting with the Miden network.
//!
//! This crate provides a lightweight client that handles connections to the Miden node, manages
//! accounts and their state, and facilitates executing, proving, and submitting transactions.
//!
//! For a protocol-level overview and guides for getting started, please visit the official
//! [Polygon Miden docs](https://0xpolygonmiden.github.io/miden-docs/).
//!
//! ## Overview
//!
//! The library is organized into several key modules:
//!
//! - **Accounts:** Provides types for managing accounts. Once accounts are tracked by the client,
//!   their state is updated with every transaction and validated during each sync.
//!
//! - **Notes:** Contains types and utilities for working with notes in the Miden client.
//!
//! - **RPC:** Facilitates communication with Miden node, exposing RPC methods for syncing state,
//!   fetching block headers, and submitting transactions.
//!
//! - **Store:** Defines and implements the persistence layer for accounts, transactions, notes, and
//!   other entities.
//!
//! - **Sync:** Provides functionality to synchronize the local state with the current state on the
//!   Miden network.
//!
//! - **Transactions:** Offers capabilities to build, execute, prove, and submit transactions.
//!
//! Additionally, the crate re-exports several utility modules:
//!
//! - **Assets:** Types and utilities for working with assets.
//! - **Auth:** Authentication-related types and functionalities.
//! - **Blocks:** Types for handling block headers.
//! - **Crypto:** Cryptographic types and utilities, including random number generators.
//! - **Utils:** Miscellaneous utilities for serialization and common operations.
//!
//! The library is designed to work in both `no_std` and `std` environments and is
//! configurable via Cargo features.
//!
//! ## Usage
//!
//! To use the Miden client library in your project, add it as a dependency in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! miden-client = "0.7.0"
//! ```
//!
//! ## Example
//!
//! Below is a brief example illustrating how to instantiate the client using the builder:
//!
//! ```rust
//! use miden_client::Client;
//!
//! # async fn create_test_client() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::initialize()
//!     .with_rpc("https://rpc.testnet.miden.io:443")
//!     .with_timeout(10_000)
//!     .with_store_path("store.sqlite3")
//!     .in_debug_mode(true)
//!     .build()
//!     .await?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! For additional usage details, configuration options, and examples, consult the documentation for
//! each module.

#![no_std]

#[macro_use]
extern crate alloc;

use alloc::boxed::Box;

#[cfg(feature = "std")]
extern crate std;

pub mod account;
pub mod note;
pub mod rpc;
pub mod store;
pub mod sync;
pub mod transaction;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub mod tests;

mod builder;
pub use builder::ClientBuilder;

mod errors;

// RE-EXPORTS
// ================================================================================================

/// Provides types and utilities for working with assets within the Miden network.
pub mod asset {
    pub use miden_objects::{
        account::delta::{
            AccountVaultDelta, FungibleAssetDelta, NonFungibleAssetDelta, NonFungibleDeltaAction,
        },
        asset::{Asset, AssetVault, FungibleAsset, NonFungibleAsset, TokenSymbol},
    };
}

/// Provides authentication-related types and functionalities for the Miden
/// network.
pub mod auth {
    pub use miden_lib::AuthScheme;
    pub use miden_objects::account::AuthSecretKey;
    pub use miden_tx::auth::{BasicAuthenticator, TransactionAuthenticator};
}

/// Provides types for working with blocks within the Miden network.
pub mod block {
    pub use miden_objects::block::BlockHeader;
}

/// Provides cryptographic types and utilities used within the Miden rollup
/// network. It re-exports commonly used types and random number generators like `FeltRng` from
/// the `miden_objects` crate.
pub mod crypto {
    pub use miden_objects::{
        crypto::{
            dsa::rpo_falcon512::SecretKey,
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

/// Provides various utilities that are commonly used throughout the Miden
/// client library.
pub mod utils {
    pub use miden_tx::utils::{
        bytes_to_hex_string, ByteReader, ByteWriter, Deserializable, DeserializationError,
        Serializable,
    };
}

/// Provides test utilities for working with accounts and account IDs
/// within the Miden network. This module is only available when the `testing` feature is
/// enabled.
#[cfg(feature = "testing")]
pub mod testing {
    pub use miden_objects::testing::*;
}

use alloc::sync::Arc;

use miden_objects::crypto::rand::{FeltRng, RpoRandomCoin};
use miden_tx::{
    auth::TransactionAuthenticator, DataStore, LocalTransactionProver, TransactionExecutor,
};
use rpc::NodeRpcClient;
use store::{data_store::ClientDataStore, Store};
use tracing::info;

// MIDEN CLIENT
// ================================================================================================

/// A light client for connecting to the Miden network.
///
/// Miden client is responsible for managing a set of accounts. Specifically, the client:
/// - Keeps track of the current and historical states of a set of accounts and related objects such
///   as notes and transactions.
/// - Connects to a Miden node to periodically sync with the current state of the network.
/// - Executes, proves, and submits transactions to the network as directed by the user.
pub struct Client<R: FeltRng = RpoRandomCoin> {
    /// The client's store, which provides a way to write and read entities to provide persistence.
    store: Arc<dyn Store>,
    /// An instance of [FeltRng] which provides randomness tools for generating new keys,
    /// serial numbers, etc.
    rng: R,
    /// An instance of [NodeRpcClient] which provides a way for the client to connect to the
    /// Miden node.
    rpc_api: Box<dyn NodeRpcClient + Send>,
    /// An instance of a [LocalTransactionProver] which will be the default prover for the client.
    tx_prover: Arc<LocalTransactionProver>,
    /// An instance of a [TransactionExecutor] that will be used to execute transactions.
    tx_executor: TransactionExecutor,
    /// Flag to enable the debug mode for scripts compilation and execution.
    in_debug_mode: bool,
}

/// Construction and access methods.
impl<R: FeltRng> Client<R> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client].
    ///
    /// ## Arguments
    ///
    /// - `rpc_api`: An instance of [NodeRpcClient] which provides a way for the client to connect to
    ///   the Miden node.
    /// - `store`: An instance of [Store], which provides a way to write and read entities to
    ///   provide persistence.
    /// - `authenticator`: Defines the transaction authenticator that will be used by the
    ///   transaction executor whenever a signature is requested from within the VM.
    /// - `in_debug_mode`: Instantiates the transaction executor (and in turn, its compiler) in
    ///   debug mode, which will enable debug logs for scripts compiled with this mode for easier
    ///   MASM debugging.
    ///
    /// # Errors
    ///
    /// Returns an error if the client couldn't be instantiated.
    pub fn new(
        rpc_api: Box<dyn NodeRpcClient + Send>,
        rng: R,
        store: Arc<dyn Store>,
        authenticator: Arc<dyn TransactionAuthenticator>,
        in_debug_mode: bool,
    ) -> Self {
        let data_store = Arc::new(ClientDataStore::new(store.clone())) as Arc<dyn DataStore>;
        let authenticator = Some(authenticator);
        let mut tx_executor = TransactionExecutor::new(data_store, authenticator);
        let tx_prover = Arc::new(LocalTransactionProver::default());

        if in_debug_mode {
            info!("Creating the Client in debug mode.");
            tx_executor = tx_executor.with_debug_mode();
        }

        Self {
            store,
            rng,
            rpc_api,
            tx_executor,
            tx_prover,
            in_debug_mode,
        }
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns true if the client is in debug mode.
    pub fn is_in_debug_mode(&self) -> bool {
        self.in_debug_mode
    }

    /// Returns a reference to the client's random number generator. This can be used to generate
    /// randomness for various purposes such as serial numbers, keys, etc.
    pub fn rng(&mut self) -> &mut R {
        &mut self.rng
    }

    // TEST HELPERS
    // --------------------------------------------------------------------------------------------

    #[cfg(any(test, feature = "testing"))]
    pub fn test_rpc_api(&mut self) -> &mut Box<dyn NodeRpcClient + Send> {
        &mut self.rpc_api
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn test_store(&mut self) -> &mut Arc<dyn Store> {
        &mut self.store
    }
}

// BUILDER ENTRY POINT (OPTIONAL HELPER)
// --------------------------------------------------------------------------------------------

impl Client<RpoRandomCoin> {
    pub fn initialize() -> ClientBuilder {
        ClientBuilder::new()
    }
}

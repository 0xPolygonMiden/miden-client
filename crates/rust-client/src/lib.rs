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
//! miden-client = "0.9"
//! ```
//!
//! ## Example
//!
//! Below is a brief example illustrating how to instantiate the client:
//!
//! ```rust
//! use std::sync::Arc;
//!
//! use miden_client::{
//!     Client, Felt,
//!     crypto::RpoRandomCoin,
//!     keystore::FilesystemKeyStore,
//!     rpc::{Endpoint, TonicRpcClient},
//!     store::{Store, sqlite_store::SqliteStore},
//! };
//! use miden_objects::crypto::rand::FeltRng;
//! use rand::{Rng, rngs::StdRng};
//!
//! # pub async fn create_test_client() -> Result<(), Box<dyn std::error::Error>> {
//! // Create the SQLite store from the client configuration.
//! let sqlite_store = SqliteStore::new("path/to/store".try_into()?).await?;
//! let store = Arc::new(sqlite_store);
//!
//! // Generate a random seed for the RpoRandomCoin.
//! let mut rng = rand::rng();
//! let coin_seed: [u64; 4] = rng.random();
//!
//! // Initialize the random coin using the generated seed.
//! let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));
//! let keystore = FilesystemKeyStore::new("path/to/keys/directory".try_into()?)?;
//!
//! // Instantiate the client using a Tonic RPC client
//! let endpoint = Endpoint::new("https".into(), "localhost".into(), Some(57291));
//! let client: Client = Client::new(
//!     Arc::new(TonicRpcClient::new(&endpoint, 10_000)),
//!     Box::new(rng),
//!     store,
//!     Arc::new(keystore),
//!     false, // Set to true for debug mode, if needed.
//! );
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
pub mod keystore;
pub mod note;
pub mod rpc;
pub mod store;
pub mod sync;
pub mod transaction;

#[cfg(feature = "std")]
pub mod builder;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub mod tests;

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
        Digest,
        crypto::{
            dsa::rpo_falcon512::SecretKey,
            merkle::{
                InOrderIndex, LeafIndex, MerklePath, MmrDelta, MmrPeaks, MmrProof, SmtLeaf,
                SmtProof,
            },
            rand::{FeltRng, RpoRandomCoin},
        },
    };
}

pub use errors::{AuthenticationError, ClientError, IdPrefixFetchError};
pub use miden_objects::{Felt, ONE, StarkField, Word, ZERO};
pub use miden_proving_service_client::proving_service::tx_prover::RemoteTransactionProver;

/// Provides various utilities that are commonly used throughout the Miden
/// client library.
pub mod utils {
    pub use miden_tx::utils::{
        ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
        bytes_to_hex_string,
        sync::{LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
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

use miden_objects::crypto::rand::FeltRng;
use miden_tx::{
    LocalTransactionProver, TransactionExecutor, TransactionMastStore,
    auth::TransactionAuthenticator,
};
use rand::RngCore;
use rpc::NodeRpcClient;
use store::{Store, data_store::ClientDataStore};
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
pub struct Client {
    /// The client's store, which provides a way to write and read entities to provide persistence.
    store: Arc<dyn Store>,
    /// An instance of [`FeltRng`] which provides randomness tools for generating new keys,
    /// serial numbers, etc.
    rng: ClientRng,
    /// An instance of [`NodeRpcClient`] which provides a way for the client to connect to the
    /// Miden node.
    rpc_api: Arc<dyn NodeRpcClient + Send>,
    /// An instance of a [`LocalTransactionProver`] which will be the default prover for the
    /// client.
    tx_prover: Arc<LocalTransactionProver>,
    /// An instance of a [`TransactionExecutor`] that will be used to execute transactions.
    tx_executor: TransactionExecutor,
    /// A MAST store, used to provide code inputs to the VM.
    mast_store: Arc<TransactionMastStore>,
    /// Flag to enable the debug mode for scripts compilation and execution.
    in_debug_mode: bool,
}

/// Construction and access methods.
impl Client {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [`Client`].
    ///
    /// ## Arguments
    ///
    /// - `api`: An instance of [`NodeRpcClient`] which provides a way for the client to connect to
    ///   the Miden node.
    /// - `store`: An instance of [`Store`], which provides a way to write and read entities to
    ///   provide persistence.
    /// - `executor_store`: An instance of [`Store`] that provides a way for [`TransactionExecutor`]
    ///   to retrieve relevant inputs at the moment of transaction execution. It should be the same
    ///   store as the one for `store`, but it doesn't have to be the **same instance**.
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
        rpc_api: Arc<dyn NodeRpcClient + Send>,
        rng: Box<dyn FeltRng>,
        store: Arc<dyn Store>,
        authenticator: Arc<dyn TransactionAuthenticator>,
        in_debug_mode: bool,
    ) -> Self {
        let client_data_store = Arc::new(ClientDataStore::new(store.clone()));
        let mast_store = client_data_store.mast_store();

        let authenticator = Some(authenticator);
        let mut tx_executor = TransactionExecutor::new(client_data_store, authenticator);
        let tx_prover = Arc::new(LocalTransactionProver::default());

        if in_debug_mode {
            info!("Creating the Client in debug mode.");
            tx_executor = tx_executor.with_debug_mode();
        }

        Self {
            store,
            rng: ClientRng::new(rng),
            rpc_api,
            tx_prover,
            tx_executor,
            in_debug_mode,
            mast_store,
        }
    }

    /// Returns true if the client is in debug mode.
    pub fn is_in_debug_mode(&self) -> bool {
        self.in_debug_mode
    }

    /// Returns a reference to the client's random number generator. This can be used to generate
    /// randomness for various purposes such as serial numbers, keys, etc.
    pub fn rng(&mut self) -> &mut ClientRng {
        &mut self.rng
    }

    // TEST HELPERS
    // --------------------------------------------------------------------------------------------

    #[cfg(any(test, feature = "testing"))]
    pub fn test_rpc_api(&mut self) -> &mut Arc<dyn NodeRpcClient + Send> {
        &mut self.rpc_api
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn test_store(&mut self) -> &mut Arc<dyn Store> {
        &mut self.store
    }
}

// CLIENT RNG
// ================================================================================================

/// A wrapper around a [`FeltRng`] that implements the [`RngCore`] trait.
/// This allows the user to pass their own generic RNG so that it's used by the client.
pub struct ClientRng(Box<dyn FeltRng>);

impl ClientRng {
    pub fn new(rng: Box<dyn FeltRng>) -> Self {
        Self(rng)
    }

    pub fn inner_mut(&mut self) -> &mut Box<dyn FeltRng> {
        &mut self.0
    }
}

impl RngCore for ClientRng {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest);
    }
}

impl FeltRng for ClientRng {
    fn draw_element(&mut self) -> Felt {
        self.0.draw_element()
    }

    fn draw_word(&mut self) -> Word {
        self.0.draw_word()
    }
}

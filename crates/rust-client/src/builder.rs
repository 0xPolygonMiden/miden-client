use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
};
use std::env::temp_dir;

use miden_objects::crypto::rand::RpoRandomCoin;
use rand::Rng;

use crate::{
    authenticator::{keystore::FilesystemKeyStore, ClientAuthenticator},
    rpc::{Endpoint, NodeRpcClient, TonicRpcClient},
    store::{sqlite_store::SqliteStore, Store},
    Client, ClientError, Felt,
};

/// A builder for constructing a Miden client.
///
/// This builder allows you to configure the various components required by the client, such as the
/// RPC endpoint, store, and RNG. It provides flexibility by letting you supply your own
/// implementations or falling back to default implementations (e.g. using a default `SQLite` store
/// and `RpoRandomCoin` for randomness) when the respective feature flags are enabled.
///
/// This builder **only exists** if the `std` feature is enabled. Otherwise,
/// it's completely ignored and never compiled.
pub struct ClientBuilder {
    /// An optional RPC endpoint.
    rpc_endpoint: Option<Endpoint>,
    /// An optional custom RPC client. If provided, this takes precedence over `rpc_endpoint`.
    rpc_api: Option<Box<dyn NodeRpcClient + Send>>,
    /// The timeout (in milliseconds) used when constructing the RPC client.
    timeout_ms: u64,
    /// An optional store provided by the user.
    store: Option<Arc<dyn Store>>,
    /// An optional RNG provided by the user.
    rng: Option<RpoRandomCoin>,
    /// The store path to use when no store is provided.
    store_path: String,
    /// An optional keystore provided by the user.
    keystore: Option<FilesystemKeyStore>,
    /// A flag to enable debug mode.
    in_debug_mode: bool,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            rpc_endpoint: None,
            rpc_api: None,
            timeout_ms: 10_000,
            store: None,
            rng: None,
            store_path: "store.sqlite3".into(),
            keystore: None,
            in_debug_mode: false,
        }
    }
}

impl ClientBuilder {
    /// Create a new `ClientBuilder` with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the RPC endpoint.
    ///
    /// Note: The RPC client is not constructed immediately. Instead, the endpoint is stored and
    /// used during the build process, together with the timeout. This means that any call to
    /// `with_timeout` will be effective as long as it happens before `build()`.
    #[must_use]
    pub fn with_rpc(mut self, endpoint: Endpoint) -> Self {
        self.rpc_endpoint = Some(endpoint);
        self
    }

    /// Sets a custom RPC client directly.
    ///
    /// This method overrides any previously set RPC endpoint.
    #[must_use]
    pub fn with_rpc_client(mut self, client: Box<dyn NodeRpcClient + Send>) -> Self {
        self.rpc_api = Some(client);
        self
    }

    /// Optionally set a custom timeout (in milliseconds) for the RPC client.
    ///
    /// This value will be used when constructing the RPC client (if one is built via `with_rpc`).
    #[must_use]
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Optionally set a custom store path.
    /// Used when no store is directly provided via `with_store()`.
    #[must_use]
    pub fn with_store_path(mut self, path: &str) -> Self {
        self.store_path = path.to_string();
        self
    }

    /// Optionally provide a store directly.
    #[must_use]
    pub fn with_store(mut self, store: Arc<dyn Store>) -> Self {
        self.store = Some(store);
        self
    }

    /// Optionally provide a custom RNG.
    #[must_use]
    pub fn with_rng(mut self, rng: RpoRandomCoin) -> Self {
        self.rng = Some(rng);
        self
    }

    /// Optionally provide a custom keystore.
    #[must_use]
    pub fn with_keystore(mut self, keystore: FilesystemKeyStore) -> Self {
        self.keystore = Some(keystore);
        self
    }

    /// Enable or disable debug mode.
    #[must_use]
    pub fn in_debug_mode(mut self, debug: bool) -> Self {
        self.in_debug_mode = debug;
        self
    }

    /// Build and return the `Client`.
    ///
    /// # Errors
    ///
    /// - Returns an error if no RPC client or endpoint was provided.
    /// - Returns an error if the store cannot be instantiated or the keystore fails.
    pub async fn build(self) -> Result<Client<RpoRandomCoin>, ClientError> {
        // Determine the RPC client to use.
        let rpc_api: Box<dyn NodeRpcClient + Send> = if let Some(client) = self.rpc_api {
            client
        } else if let Some(endpoint) = self.rpc_endpoint {
            Box::new(TonicRpcClient::new(&endpoint, self.timeout_ms))
        } else {
            return Err(ClientError::ClientInitializationError(
                "RPC client or endpoint is required. Call `.with_rpc(...)` or `.with_rpc_client(...)`.".into(),
            ));
        };

        // If no store was provided, create a SQLite store from the given path.
        let arc_store: Arc<dyn Store> = if let Some(store) = self.store {
            store
        } else {
            let store = SqliteStore::new(self.store_path.clone().into())
                .await
                .map_err(ClientError::StoreError)?;
            Arc::new(store)
        };

        // Use the provided RNG, or create a default one.
        let rng = if let Some(user_rng) = self.rng {
            user_rng
        } else {
            let mut seed_rng = rand::thread_rng();
            let coin_seed: [u64; 4] = seed_rng.gen();
            RpoRandomCoin::new(coin_seed.map(Felt::new))
        };

        // Use the provided keystore, or default to a FilesystemKeyStore with temp_dir.
        let keystore = match self.keystore {
            Some(ks) => ks,
            None => FilesystemKeyStore::new(temp_dir())
                .map_err(|err| ClientError::ClientInitializationError(err.to_string()))?,
        };

        let authenticator = ClientAuthenticator::new(rng, keystore);

        Ok(Client::new(
            rpc_api,
            rng,
            arc_store,
            Arc::new(authenticator),
            self.in_debug_mode,
        ))
    }
}

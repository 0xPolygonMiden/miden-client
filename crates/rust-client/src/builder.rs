use alloc::{
    string::{String, ToString},
    sync::Arc,
};

use miden_objects::crypto::rand::RpoRandomCoin;
use rand::Rng;

use crate::{
    authenticator::{keystore::FilesystemKeyStore, ClientAuthenticator},
    rpc::{Endpoint, NodeRpcClient, TonicRpcClient},
    store::{sqlite_store::SqliteStore, Store},
    Client, ClientError, Felt,
};

/// Represents the configuration for a keystore.
///
/// The purpose of this enum is to delay the actual instantiation of the keystore until the build
/// phase. This allows the builder to accept either:
///
/// - A direct instance of a `FilesystemKeyStore`, or
/// - A keystore path as a string which is then used to initialize the keystore during `build()`.
///
/// Without this enum, we are forced to perform the initialization immediately, which would
/// require unwrapping (and potentially panicking) if initialization fails. By deferring it, we
/// allow error handling during `build()`.
enum KeystoreConfig {
    Path(String),
    Instance(FilesystemKeyStore),
}

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
    rpc_api: Option<Arc<dyn NodeRpcClient + Send>>,
    /// The timeout (in milliseconds) used when constructing the RPC client.
    timeout_ms: u64,
    /// An optional store provided by the user.
    store: Option<Arc<dyn Store>>,
    /// An optional RNG provided by the user.
    rng: Option<RpoRandomCoin>,
    /// The store path to use when no store is directly provided via `with_store()`.
    store_path: String,
    /// The keystore configuration provided by the user.
    keystore: Option<KeystoreConfig>,
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

    /// Sets a custom RPC client directly.
    ///
    /// This method overrides any previously set RPC endpoint.
    #[must_use]
    pub fn with_rpc(mut self, client: Arc<dyn NodeRpcClient + Send>) -> Self {
        self.rpc_api = Some(client);
        self
    }

    /// Sets the RPC endpoint.
    ///
    /// Note: The RPC client is not constructed immediately. Instead, the endpoint is stored and
    /// used during the build process, together with the timeout. This means that any call to
    /// `with_timeout` will be effective as long as it happens before `build()`.
    #[must_use]
    pub fn with_tonic_rpc(mut self, endpoint: Endpoint) -> Self {
        self.rpc_endpoint = Some(endpoint);
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
    pub fn with_sqlite_store(mut self, path: &str) -> Self {
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

    /// Optionally provide a custom keystore instance.
    ///
    /// This implementation accepts an already constructed `FilesystemKeyStore` and stores it for
    /// later use during client initialization.
    #[must_use]
    pub fn with_keystore(mut self, keystore: FilesystemKeyStore) -> Self {
        self.keystore = Some(KeystoreConfig::Instance(keystore));
        self
    }

    /// **Required:** Provide the keystore path as a string.
    ///
    /// This method stores the keystore path as a configuration option so that the actual keystore
    /// initialization is deferred until `build()`. This prevents the need to unwrap errors during
    /// builder chaining.
    #[must_use]
    pub fn with_filesystem_keystore(mut self, keystore_path: &str) -> Self {
        self.keystore = Some(KeystoreConfig::Path(keystore_path.to_string()));
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
    /// - Returns an error if the store cannot be instantiated.
    /// - Returns an error if the keystore is not specified or fails to initialize.
    pub async fn build(self) -> Result<Client<RpoRandomCoin>, ClientError> {
        // Determine the RPC client to use.
        let rpc_api: Arc<dyn NodeRpcClient + Send> = if let Some(client) = self.rpc_api {
            client
        } else if let Some(endpoint) = self.rpc_endpoint {
            Arc::new(TonicRpcClient::new(&endpoint, self.timeout_ms))
        } else {
            return Err(ClientError::ClientInitializationError(
                "RPC client or endpoint is required. Call `.with_rpc(...)` or `.with_rpc_client(...)`."
                    .into(),
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

        // Require a keystore to be specified.
        let keystore = match self.keystore {
            Some(KeystoreConfig::Instance(k)) => k,
            Some(KeystoreConfig::Path(ref path)) => FilesystemKeyStore::new(path.into())
                .map_err(|err| ClientError::ClientInitializationError(err.to_string()))?,
            None => {
                return Err(ClientError::ClientInitializationError(
                    "Keystore must be specified. Call `.with_keystore(...)` or `.with_filesystem_keystore(...)` with a keystore path."
                        .into(),
                ))
            }
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

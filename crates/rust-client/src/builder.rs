use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
};

use miden_objects::{Felt, crypto::rand::RpoRandomCoin};
use miden_tx::auth::TransactionAuthenticator;
use rand::{Rng, rngs::StdRng};

use crate::{
    Client, ClientError,
    keystore::FilesystemKeyStore,
    rpc::{Endpoint, NodeRpcClient, TonicRpcClient},
    store::{Store, sqlite_store::SqliteStore},
};

/// Represents the configuration for an authenticator.
///
/// This enum defers authenticator instantiation until the build phase. The builder can accept
/// either:
///
/// - A direct instance of an authenticator, or
/// - A keystore path as a string which is then used as an authenticator.
enum AuthenticatorConfig {
    Path(String),
    Instance(Arc<dyn TransactionAuthenticator>),
}

/// A builder for constructing a Miden client.
///
/// This builder allows you to configure the various components required by the client, such as the
/// RPC endpoint, store, RNG, and keystore. It is generic over the keystore type. By default, it
/// uses `FilesystemKeyStore<rand::rngs::StdRng>`.
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
    /// The store path to use when no store is directly provided via `with_store()`.
    store_path: String,
    /// The keystore configuration provided by the user.
    keystore: Option<AuthenticatorConfig>,
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

    /// Enable or disable debug mode.
    #[must_use]
    pub fn in_debug_mode(mut self, debug: bool) -> Self {
        self.in_debug_mode = debug;
        self
    }

    /// Sets a custom RPC client directly.
    #[must_use]
    pub fn with_rpc(mut self, client: Box<dyn NodeRpcClient + Send>) -> Self {
        self.rpc_api = Some(client);
        self
    }

    /// Sets the RPC endpoint.
    #[must_use]
    pub fn with_tonic_rpc(mut self, endpoint: Endpoint) -> Self {
        self.rpc_endpoint = Some(endpoint);
        self
    }

    /// Optionally set a custom timeout (in milliseconds) for the RPC client.
    #[must_use]
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Optionally set a custom store path.
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

    /// Optionally provide a custom authenticator instance.
    #[must_use]
    pub fn with_authenticator(mut self, authenticator: Arc<dyn TransactionAuthenticator>) -> Self {
        self.keystore = Some(AuthenticatorConfig::Instance(authenticator));
        self
    }
}

/// Methods that only make sense when using the default keystore type,
/// i.e. `FilesystemKeyStore<rand::rngs::StdRng>`.
impl ClientBuilder {
    /// **Required:** Provide the keystore path as a string.
    ///
    /// This stores the keystore path as a configuration option so that actual keystore
    /// initialization is deferred until `build()`. This avoids panicking during builder chaining.
    #[must_use]
    pub fn with_filesystem_keystore(mut self, keystore_path: &str) -> Self {
        self.keystore = Some(AuthenticatorConfig::Path(keystore_path.to_string()));
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
        let rpc_api: Box<dyn NodeRpcClient + Send> = if let Some(client) = self.rpc_api {
            client
        } else if let Some(endpoint) = self.rpc_endpoint {
            Box::new(TonicRpcClient::new(&endpoint, self.timeout_ms))
        } else {
            return Err(ClientError::ClientInitializationError(
                "RPC client or endpoint is required. Call `.with_rpc(...)` or `.with_tonic_rpc(...)`."
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
            let coin_seed: [u64; 4] = seed_rng.r#gen();
            RpoRandomCoin::new(coin_seed.map(Felt::new))
        };

        // Initialize the authenticator.
        let authenticator = match self.keystore {
            Some(AuthenticatorConfig::Instance(authenticator)) => authenticator,
            Some(AuthenticatorConfig::Path(ref path)) => {
                let fs_keystore = FilesystemKeyStore::<StdRng>::new(path.into())
                    .map_err(|err| ClientError::ClientInitializationError(err.to_string()))?;

                Arc::new(fs_keystore)
            },
            None => {
                return Err(ClientError::ClientInitializationError(
                    "Keystore must be specified. Call `.with_keystore(...)` or `.with_filesystem_keystore(...)` with a keystore path."
                        .into(),
                ))
            }
        };

        Ok(Client::new(rpc_api, rng, arc_store, authenticator, self.in_debug_mode))
    }
}

use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
};

use crate::{
    rpc::{Endpoint, NodeRpcClient},
    store::{Store, StoreAuthenticator},
    Client, ClientError, Felt,
};

#[cfg(feature = "tonic")]
use crate::rpc::TonicRpcClient;

#[cfg(feature = "sqlite")]
use crate::store::sqlite_store::SqliteStore;

use miden_objects::crypto::rand::RpoRandomCoin;
use rand::Rng;

/// A builder for constructing a Miden client.
///
/// This builder allows you to configure the various components required by the client, such as the RPC endpoint,
/// store, and RNG. It provides flexibility by letting you supply your own implementations or falling back to default
/// implementations (e.g. using a default SQLite store and `RpoRandomCoin` for randomness) when the respective feature
/// flags are enabled.
pub struct ClientBuilder {
    /// An optional RPC client implementing `NodeRpcClient + Send`.
    rpc_api: Option<Box<dyn NodeRpcClient + Send>>,
    /// The timeout (in milliseconds) used when connecting to the RPC endpoint.
    timeout_ms: u64,
    /// An optional store provided by the user.
    /// If not provided and the `sqlite` feature is enabled, a default SQLite store will be created using `store_path`.
    store: Option<Arc<dyn Store>>,
    /// An optional RNG provided by the user.
    /// If not provided, a default `RpoRandomCoin` will be created.
    rng: Option<RpoRandomCoin>,
    /// The store path to use when no store is provided.
    store_path: String,
    /// A flag to enable debug mode.
    /// When set to `true`, debug logging and behavior may be enabled.
    in_debug_mode: bool,
}

impl ClientBuilder {
    /// Starts the builder with default values.
    pub fn new() -> Self {
        Self {
            rpc_api: None,
            timeout_ms: 10_000,
            store: None,
            rng: None,
            store_path: "store.sqlite3".into(),
            in_debug_mode: false,
        }
    }

    /// Sets the RPC client directly.
    pub fn with_rpc_client(mut self, client: Box<dyn NodeRpcClient + Send>) -> Self {
        self.rpc_api = Some(client);
        self
    }

    /// Sets the RPC endpoint via a URL.
    ///
    /// This method is available only when the `tonic` feature is enabled.
    /// It parses the provided URL to extract the scheme, host, and port, and creates a `TonicRpcClient`.
    #[cfg(feature = "tonic")]
    pub fn with_rpc(mut self, url: &str) -> Self {
        let endpoint = Endpoint::try_from(url).unwrap();
        self.rpc_api = Some(Box::new(TonicRpcClient::new(endpoint, self.timeout_ms)));
        self
    }

    /// Optionally set a custom timeout (in ms).
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Optionally set a custom store path.
    ///
    /// This path will be used to create a default SQLite store if no store is provided via `with_store()`
    /// and if the `sqlite` feature is enabled.
    pub fn with_store_path(mut self, path: &str) -> Self {
        self.store_path = path.to_string();
        self
    }

    /// Optionally provide a store directly.
    pub fn with_store(mut self, store: Arc<dyn Store>) -> Self {
        self.store = Some(store);
        self
    }

    /// Optionally provide a custom RNG.
    pub fn with_rng(mut self, rng: RpoRandomCoin) -> Self {
        self.rng = Some(rng);
        self
    }

    /// Optionally enable debug mode.
    pub fn in_debug_mode(mut self, debug: bool) -> Self {
        self.in_debug_mode = debug;
        self
    }

    /// Builds the Client.
    ///
    /// If not all components are provided by the user, default implementations are used.
    /// For example, if no store is provided and the `sqlite` feature is enabled, a SQLite store will be created
    /// using `store_path`; if no RNG is provided, a default `RpoRandomCoin` is generated.
    pub async fn build(self) -> Result<Client<RpoRandomCoin>, ClientError> {
        // Ensure an RPC client was provided.
        let rpc_api = self.rpc_api.ok_or_else(|| {
            ClientError::ClientInitializationError(
                "RPC client must be provided. Use with_rpc() or with_rpc_client() to set one."
                    .into(),
            )
        })?;

        // Set up the store.
        // If the user provided a store, use it.
        // Otherwise, if the `sqlite` feature is enabled, build one from the store_path.
        // If not, return an error.
        let arc_store: Arc<dyn Store> = {
            #[cfg(feature = "sqlite")]
            {
                if let Some(store) = self.store {
                    store
                } else {
                    let store = SqliteStore::new(self.store_path.into())
                        .await
                        .map_err(ClientError::StoreError)?;
                    Arc::new(store)
                }
            }
            #[cfg(not(feature = "sqlite"))]
            {
                self.store.ok_or_else(|| {
                    ClientError::ClientInitializationError(
                        "No store provided and the sqlite feature is disabled.".into(),
                    )
                })?
            }
        };

        // Set up the RNG.
        // Use the user-provided RNG if available; otherwise, build a default one.
        let rng = if let Some(rng) = self.rng {
            rng
        } else {
            let mut seed_rng = rand::thread_rng();
            let coin_seed: [u64; 4] = seed_rng.gen();
            RpoRandomCoin::new(coin_seed.map(Felt::new))
        };

        // Create the authenticator.
        let authenticator = StoreAuthenticator::new_with_rng(arc_store.clone(), rng.clone());

        // Create the client.
        let client =
            Client::new(rpc_api, rng, arc_store, Arc::new(authenticator), self.in_debug_mode);
        Ok(client)
    }
}

use crate::alloc::string::ToString;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;

use crate::{
    rpc::{Endpoint, NodeRpcClient, TonicRpcClient},
    store::{sqlite_store::SqliteStore, StoreAuthenticator},
    Client, ClientError, Felt,
};
use miden_objects::crypto::rand::RpoRandomCoin;
use rand::Rng;

pub struct ClientBuilder {
    rpc_api: Option<Box<dyn NodeRpcClient + Send>>,
    timeout_ms: u64,
    store_path: Option<String>,
    in_debug_mode: bool,
}

impl ClientBuilder {
    /// Starts the builder with default values.
    pub fn new() -> Self {
        Self {
            rpc_api: None,
            timeout_ms: 10_000,
            store_path: Some("store.sqlite3".into()),
            in_debug_mode: false,
        }
    }

    /// Sets the RPC endpoint via a URL.
    pub fn with_rpc(mut self, url: &str) -> Self {
        // Determine the scheme and strip it from the URL.
        let (scheme, rest) = if let Some(stripped) = url.strip_prefix("https://") {
            ("https", stripped)
        } else if let Some(stripped) = url.strip_prefix("http://") {
            ("http", stripped)
        } else {
            ("https", url)
        };

        // Attempt to find a colon indicating a port.
        let (host, port) = if let Some(colon_index) = rest.find(':') {
            // Split the host and port.
            let host = &rest[..colon_index];
            let port_str = &rest[colon_index + 1..];
            // Try parsing the port. If it fails, use None.
            let port = port_str.parse::<u16>().ok();
            (host.to_string(), port)
        } else {
            // No colon found, so use the entire string as the host and no port.
            (rest.to_string(), None)
        };

        // Create the endpoint using the parsed scheme, host, and port.
        let endpoint = Endpoint::new(scheme.to_string(), host, port);
        self.rpc_api = Some(Box::new(TonicRpcClient::new(endpoint, self.timeout_ms)));
        self
    }

    /// Optionally set a custom timeout (in ms).
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Optionally set a custom store path.
    pub fn with_store_path(mut self, path: &str) -> Self {
        self.store_path = Some(path.to_string());
        self
    }

    /// Optionally enable debug mode.
    pub fn in_debug_mode(mut self, debug: bool) -> Self {
        self.in_debug_mode = debug;
        self
    }

    /// Builds the Client.
    pub async fn build(self) -> Result<Client<RpoRandomCoin>, ClientError> {
        // Set up the RPC client. If not provided, return an error.
        let rpc_api = self.rpc_api.ok_or_else(|| {
            ClientError::ClientInitializationError(
                "RPC client must be provided. Use with_rpc() to set one.".into(),
            )
        })?;

        let store_path = self.store_path.unwrap_or_else(|| "store.sqlite3".into());
        let store = SqliteStore::new(store_path.into()).await.map_err(ClientError::StoreError)?;
        let arc_store = Arc::new(store);

        let mut seed_rng = rand::thread_rng();
        let coin_seed: [u64; 4] = seed_rng.gen();
        let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));

        let authenticator = StoreAuthenticator::new_with_rng(arc_store.clone(), rng.clone());

        let client =
            Client::new(rpc_api, rng, arc_store, Arc::new(authenticator), self.in_debug_mode);
        Ok(client)
    }
}

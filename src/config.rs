use core::fmt;
use std::path::PathBuf;

use serde::Deserialize;

// CLIENT CONFIG
// ================================================================================================

/// Configuration options of Miden client.
#[derive(Debug, Default, Deserialize, Eq, PartialEq)]
pub struct ClientConfig {
    /// Describes settings related to the store.
    pub store: StoreConfig,
    /// Describes settings related to the RPC endpoint
    pub rpc: RpcConfig,
}

impl ClientConfig {
    /// Returns a new instance of [ClientConfig] with the specified store path and node endpoint.
    pub const fn new(store: StoreConfig, rpc: RpcConfig) -> Self {
        Self { store, rpc }
    }
}

// ENDPOINT
// ================================================================================================

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub struct Endpoint {
    protocol: String,
    host: String,
    port: u16,
}

impl Endpoint {
    /// Returns a new instance of [Endpoint] with the specified protocol, host, and port.
    pub const fn new(protocol: String, host: String, port: u16) -> Self {
        Self {
            protocol,
            host,
            port,
        }
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}:{}", self.protocol, self.host, self.port)
    }
}

impl Default for Endpoint {
    fn default() -> Self {
        const MIDEN_NODE_PORT: u16 = 57291;

        Self {
            protocol: "http".to_string(),
            host: "localhost".to_string(),
            port: MIDEN_NODE_PORT,
        }
    }
}

// STORE CONFIG
// ================================================================================================

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct StoreConfig {
    pub database_filepath: String,
}

impl From<&ClientConfig> for StoreConfig {
    fn from(config: &ClientConfig) -> Self {
        Self {
            database_filepath: config.store.database_filepath.clone(),
        }
    }
}

impl From<&str> for StoreConfig {
    fn from(value: &str) -> Self {
        Self {
            database_filepath: value.to_string(),
        }
    }
}

impl From<String> for StoreConfig {
    fn from(value: String) -> Self {
        Self {
            database_filepath: value,
        }
    }
}

impl Default for StoreConfig {
    fn default() -> Self {
        const STORE_FILENAME: &str = "store.sqlite3";

        // get directory of the currently executing binary, or fallback to the current directory
        let exec_dir = match std::env::current_exe() {
            Ok(mut path) => {
                path.pop();
                path
            }
            Err(_) => PathBuf::new(),
        };

        let database_filepath = exec_dir
            .join(STORE_FILENAME)
            .into_os_string()
            .into_string()
            .expect("Creating the hardcoded store path should not panic");

        Self { database_filepath }
    }
}

// RPC CONFIG
// ================================================================================================

#[derive(Debug, Default, Deserialize, Eq, PartialEq)]
pub struct RpcConfig {
    /// Address of the Miden node to connect to.
    pub endpoint: Endpoint,
}

impl From<Endpoint> for RpcConfig {
    fn from(value: Endpoint) -> Self {
        Self { endpoint: value }
    }
}

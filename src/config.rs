use core::fmt;
use std::path::PathBuf;

// CLIENT CONFIG
// ================================================================================================

/// Configuration options of Miden client.
#[derive(Debug, PartialEq, Eq)]
pub struct ClientConfig {
    /// Location of the client's data file.
    pub store_path: String,
    /// Address of the Miden node to connect to.
    pub node_endpoint: Endpoint,
}

impl ClientConfig {
    /// Returns a new instance of [ClientConfig] with the specified store path and node endpoint.
    pub const fn new(store_path: String, node_endpoint: Endpoint) -> Self {
        Self {
            store_path,
            node_endpoint,
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        #[cfg(not(any(test, feature = "mock")))]
        const STORE_FILENAME: &str = "store.sqlite3";

        #[cfg(any(test, feature = "mock"))]
        const STORE_FILENAME: &str = "test.store.sqlite3";

        // get directory of the currently executing binary, or fallback to the current directory
        let exec_dir = match std::env::current_exe() {
            Ok(mut path) => {
                path.pop();
                path
            }
            Err(_) => PathBuf::new(),
        };

        let store_path = exec_dir.join(STORE_FILENAME);

        Self {
            store_path: store_path
                .into_os_string()
                .into_string()
                .expect("Creating the hardcoded path to the store file should not panic"),
            node_endpoint: Endpoint::default(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
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

pub struct StoreConfig {
    pub path: String,
}

impl From<&ClientConfig> for StoreConfig {
    fn from(config: &ClientConfig) -> Self {
        Self {
            path: config.store_path.clone(),
        }
    }
}

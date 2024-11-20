use core::fmt::Debug;
use std::{
    fmt,
    path::{Path, PathBuf},
};

use figment::{
    value::{Dict, Map},
    Metadata, Profile, Provider,
};
use serde::{Deserialize, Serialize};

const TOKEN_SYMBOL_MAP_FILEPATH: &str = "token_symbol_map.toml";

// CLI CONFIG
// ================================================================================================

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CliConfig {
    /// Describes settings related to the RPC endpoint
    pub rpc: RpcConfig,
    /// Describes settings related to the store.
    pub store: SqliteStoreConfig,
    /// Address of the Miden node to connect to.
    pub default_account_id: Option<String>,
    /// Path to the file containing the token symbol map.
    pub token_symbol_map_filepath: PathBuf,
    /// RPC endpoint for the proving service. If this is not present, a local prover will be used.
    pub remote_prover_endpoint: Option<Endpoint>,
}

// Make `ClientConfig` a provider itself for composability.
impl Provider for CliConfig {
    fn metadata(&self) -> Metadata {
        Metadata::named("CLI Config")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        figment::providers::Serialized::defaults(CliConfig::default()).data()
    }

    fn profile(&self) -> Option<Profile> {
        // Optionally, a profile that's selected by default.
        None
    }
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            rpc: RpcConfig::default(),
            store: SqliteStoreConfig::default(),
            default_account_id: None,
            token_symbol_map_filepath: Path::new(TOKEN_SYMBOL_MAP_FILEPATH).to_path_buf(),
            remote_prover_endpoint: None,
        }
    }
}

// ENDPOINT
// ================================================================================================

/// The `Endpoint` struct represents a network endpoint, consisting of a protocol, a host, and a
/// port.
///
/// This struct is used to define the address of a Miden node that the client will connect to.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Endpoint {
    /// The protocol used to connect to the endpoint (e.g., "http", "https").
    pub protocol: String,
    /// The hostname or IP address of the endpoint.
    pub host: String,
    /// The port number of the endpoint.
    pub port: u16,
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}:{}", self.protocol, self.host, self.port)
    }
}

const MIDEN_NODE_PORT: u16 = 57291;
impl Default for Endpoint {
    fn default() -> Self {
        Self {
            protocol: "http".to_string(),
            host: "localhost".to_string(),
            port: MIDEN_NODE_PORT,
        }
    }
}

impl TryFrom<&str> for Endpoint {
    type Error = String;

    fn try_from(endpoint: &str) -> Result<Self, Self::Error> {
        let protocol_separator_index = endpoint.find("://");
        let port_separator_index = endpoint.rfind(':');

        // port separator index might match with the protocol separator, if so that means there was
        // no port defined
        let port_separator_index = if port_separator_index == protocol_separator_index {
            None
        } else {
            port_separator_index
        };

        let (protocol, hostname, port) = match (protocol_separator_index, port_separator_index) {
            (Some(protocol_idx), Some(port_idx)) => {
                let (protocol_and_hostname, port) = endpoint.split_at(port_idx);
                let port = port[1..].parse::<u16>().map_err(|err| err.to_string())?;

                let (protocol, hostname) = protocol_and_hostname.split_at(protocol_idx);
                // skip the separator
                let hostname = &hostname[3..];

                (protocol, hostname, port)
            },
            (Some(protocol_idx), None) => {
                let (protocol, hostname) = endpoint.split_at(protocol_idx);
                // skip the separator
                let hostname = &hostname[3..];

                (protocol, hostname, MIDEN_NODE_PORT)
            },
            (None, Some(port_idx)) => {
                let (hostname, port) = endpoint.split_at(port_idx);
                let port = port[1..].parse::<u16>().map_err(|err| err.to_string())?;

                ("https", hostname, port)
            },
            (None, None) => ("https", endpoint, MIDEN_NODE_PORT),
        };

        Ok(Endpoint {
            protocol: protocol.to_string(),
            host: hostname.to_string(),
            port,
        })
    }
}

// RPC CONFIG
// ================================================================================================

/// Settings for the RPC client.
#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RpcConfig {
    /// Address of the Miden node to connect to.
    pub endpoint: Endpoint,
    /// Timeout for the RPC api requests, in milliseconds.
    pub timeout_ms: u64,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            endpoint: Endpoint::default(),
            timeout_ms: 10000,
        }
    }
}

// STORE CONFIG
// ================================================================================================

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SqliteStoreConfig {
    pub database_filepath: PathBuf,
}

impl TryFrom<&str> for SqliteStoreConfig {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        SqliteStoreConfig::try_from(value.to_string())
    }
}

// TODO: Implement error checking for invalid paths, or make it based on Path types
impl TryFrom<String> for SqliteStoreConfig {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self { database_filepath: PathBuf::from(value) })
    }
}

impl Default for SqliteStoreConfig {
    fn default() -> Self {
        const STORE_FILENAME: &str = "store.sqlite3";

        // Get current directory
        let exec_dir = PathBuf::new();

        // Append filepath
        let database_filepath = exec_dir.join(STORE_FILENAME);

        Self { database_filepath }
    }
}

#[cfg(test)]
mod test {
    use crate::config::{Endpoint, MIDEN_NODE_PORT};

    #[test]
    fn test_endpoint_parsing_with_hostname_only() {
        let endpoint = Endpoint::try_from("some.test.domain").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "https".to_string(),
            host: "some.test.domain".to_string(),
            port: MIDEN_NODE_PORT,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_ip() {
        let endpoint = Endpoint::try_from("192.168.0.1").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "https".to_string(),
            host: "192.168.0.1".to_string(),
            port: MIDEN_NODE_PORT,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_port() {
        let endpoint = Endpoint::try_from("some.test.domain:8000").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "https".to_string(),
            host: "some.test.domain".to_string(),
            port: 8000,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_ip_and_port() {
        let endpoint = Endpoint::try_from("192.168.0.1:8000").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "https".to_string(),
            host: "192.168.0.1".to_string(),
            port: 8000,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_protocol() {
        let endpoint = Endpoint::try_from("hkttp://some.test.domain").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "hkttp".to_string(),
            host: "some.test.domain".to_string(),
            port: MIDEN_NODE_PORT,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_protocol_and_ip() {
        let endpoint = Endpoint::try_from("http://192.168.0.1").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "http".to_string(),
            host: "192.168.0.1".to_string(),
            port: MIDEN_NODE_PORT,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_both_protocol_and_port() {
        let endpoint = Endpoint::try_from("http://some.test.domain:8080").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "http".to_string(),
            host: "some.test.domain".to_string(),
            port: 8080,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_ip_and_protocol_and_port() {
        let endpoint = Endpoint::try_from("http://192.168.0.1:8080").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "http".to_string(),
            host: "192.168.0.1".to_string(),
            port: 8080,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_should_fail_for_invalid_port() {
        let endpoint = Endpoint::try_from("some.test.domain:8000/hello");
        assert!(endpoint.is_err());
    }
}

use core::fmt::Debug;
use std::path::{Path, PathBuf};

use figment::{
    value::{Dict, Map},
    Metadata, Profile, Provider,
};
use miden_client::{
    config::{Endpoint, RpcConfig},
    store::sqlite_store::config::SqliteStoreConfig,
};
use serde::{Deserialize, Serialize};

const TOKEN_SYMBOL_MAP_FILEPATH: &str = "token_symbol_map.toml";

// CLI CONFIG
// ================================================================================================

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CliConfig {
    /// Describes settings related to the RPC endpoint
    pub rpc: CliRpcConfig,
    /// Describes settings related to the store.
    pub store: CliSqliteStoreConfig,
    /// Address of the Miden node to connect to.
    pub default_account_id: Option<String>,
    /// Path to the file containing the token symbol map.
    pub token_symbol_map_filepath: PathBuf,
    /// RPC endpoint for the proving service. If this is not present, a local prover will be used.
    pub remote_prover_endpoint: Option<CliEndpoint>,
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
            rpc: CliRpcConfig::default(),
            store: CliSqliteStoreConfig::default(),
            default_account_id: None,
            token_symbol_map_filepath: Path::new(TOKEN_SYMBOL_MAP_FILEPATH).to_path_buf(),
            remote_prover_endpoint: None,
        }
    }
}

// RPC CONFIG
// ================================================================================================
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct CliEndpoint {
    /// The protocol used to connect to the endpoint (e.g., "http", "https").
    pub protocol: String,
    /// The hostname or IP address of the endpoint.
    pub host: String,
    /// The port number of the endpoint.
    pub port: u16,
}

impl From<CliEndpoint> for Endpoint {
    fn from(endpoint: CliEndpoint) -> Self {
        Endpoint::new(endpoint.protocol, endpoint.host, endpoint.port)
    }
}

impl From<Endpoint> for CliEndpoint {
    fn from(endpoint: Endpoint) -> Self {
        Self {
            protocol: endpoint.protocol().to_string(),
            host: endpoint.host().to_string(),
            port: endpoint.port(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CliRpcConfig {
    /// Address of the Miden node to connect to.
    pub endpoint: CliEndpoint,
    /// Timeout for the RPC api requests, in milliseconds.
    pub timeout_ms: u64,
}

impl From<RpcConfig> for CliRpcConfig {
    fn from(config: RpcConfig) -> Self {
        Self {
            endpoint: config.endpoint.into(),
            timeout_ms: config.timeout_ms,
        }
    }
}

impl From<CliRpcConfig> for RpcConfig {
    fn from(config: CliRpcConfig) -> Self {
        Self {
            endpoint: config.endpoint.into(),
            timeout_ms: config.timeout_ms,
        }
    }
}

impl Default for CliRpcConfig {
    fn default() -> Self {
        RpcConfig::default().into()
    }
}

// STORE CONFIG
// ================================================================================================
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CliSqliteStoreConfig {
    pub database_filepath: String,
}

impl From<SqliteStoreConfig> for CliSqliteStoreConfig {
    fn from(config: SqliteStoreConfig) -> Self {
        Self {
            database_filepath: config.database_filepath,
        }
    }
}

impl From<CliSqliteStoreConfig> for SqliteStoreConfig {
    fn from(config: CliSqliteStoreConfig) -> Self {
        Self {
            database_filepath: config.database_filepath,
        }
    }
}

impl Default for CliSqliteStoreConfig {
    fn default() -> Self {
        SqliteStoreConfig::default().into()
    }
}

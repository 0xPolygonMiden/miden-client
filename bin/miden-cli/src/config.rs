use core::fmt::Debug;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use figment::{
    value::{Dict, Map},
    Metadata, Profile, Provider,
};
use miden_client::rpc::Endpoint;
use serde::{Deserialize, Serialize};

const TOKEN_SYMBOL_MAP_FILEPATH: &str = "token_symbol_map.toml";

// CLI CONFIG
// ================================================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct CliConfig {
    /// Describes settings related to the RPC endpoint.
    pub rpc: RpcConfig,
    /// Path to the sqlite store file.
    pub store_filepath: PathBuf,
    /// Path to authenticator file.
    pub authenticator_filepath: PathBuf,
    /// Address of the Miden node to connect to.
    pub default_account_id: Option<String>,
    /// Path to the file containing the token symbol map.
    pub token_symbol_map_filepath: PathBuf,
    /// RPC endpoint for the proving service. If this isn't present, a local prover will be used.
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
        const STORE_FILENAME: &str = "store.sqlite3";
        const AUTHENTICATOR_FILENAME: &str = "keypairs.txt"; //TODO: revisit default name

        // Get current directory
        let exec_dir = PathBuf::new();

        Self {
            rpc: RpcConfig::default(),
            store_filepath: exec_dir.join(STORE_FILENAME),
            authenticator_filepath: exec_dir.join(AUTHENTICATOR_FILENAME),
            default_account_id: None,
            token_symbol_map_filepath: Path::new(TOKEN_SYMBOL_MAP_FILEPATH).to_path_buf(),
            remote_prover_endpoint: None,
        }
    }
}

// RPC CONFIG
// ================================================================================================

/// Settings for the RPC client.
#[derive(Debug, Deserialize, Serialize)]
pub struct RpcConfig {
    /// Address of the Miden node to connect to.
    pub endpoint: CliEndpoint,
    /// Timeout for the RPC api requests, in milliseconds.
    pub timeout_ms: u64,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            endpoint: Endpoint::default().into(),
            timeout_ms: 10000,
        }
    }
}

// CLI ENDPOINT
// ================================================================================================

#[derive(Clone, Debug)]
pub struct CliEndpoint(pub Endpoint);

impl Display for CliEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for CliEndpoint {
    type Error = String;

    fn try_from(endpoint: &str) -> Result<Self, Self::Error> {
        let endpoint = Endpoint::try_from(endpoint).map_err(|err| err.to_string())?;
        Ok(Self(endpoint))
    }
}

impl From<Endpoint> for CliEndpoint {
    fn from(endpoint: Endpoint) -> Self {
        Self(endpoint)
    }
}

impl From<CliEndpoint> for Endpoint {
    fn from(endpoint: CliEndpoint) -> Self {
        endpoint.0
    }
}

impl Serialize for CliEndpoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for CliEndpoint {
    fn deserialize<D>(deserializer: D) -> Result<CliEndpoint, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let endpoint = String::deserialize(deserializer)?;
        CliEndpoint::try_from(endpoint.as_str()).map_err(serde::de::Error::custom)
    }
}

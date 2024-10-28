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
    pub rpc: RpcConfig,
    /// Describes settings related to the store.
    pub store: SqliteStoreConfig,
    /// Address of the Miden node to connect to.
    pub default_account_id: Option<String>,
    /// Path to the file containing the token symbol map.
    pub token_symbol_map_filepath: PathBuf,
    /// RPC endpoint for the proving service. If this is not present, a local prover will be used.
    pub proving_rpc_endpoint: Option<Endpoint>,
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
            proving_rpc_endpoint: None,
        }
    }
}

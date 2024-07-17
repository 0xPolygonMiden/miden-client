use core::fmt::Debug;

use figment::{
    value::{Dict, Map},
    Metadata, Profile, Provider,
};
use miden_client::{config::RpcConfig, store::sqlite_store::config::SqliteStoreConfig};
use serde::{Deserialize, Serialize};

const TOKEN_SYMBOL_MAPPINGS_FILE_NAME: &str = "token_symbol_mappings.toml";

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
    /// Path to the file containing token symbol mappings.
    pub token_symbol_mappings_file: String,
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
            token_symbol_mappings_file: TOKEN_SYMBOL_MAPPINGS_FILE_NAME.to_string(),
        }
    }
}

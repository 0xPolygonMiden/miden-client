use core::fmt::Debug;

use figment::{
    value::{Dict, Map},
    Metadata, Profile, Provider,
};
use miden_client::{config::RpcConfig, store::sqlite_store::config::SqliteStoreConfig};
use serde::{Deserialize, Serialize};

// CLI CONFIG
// ================================================================================================

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct CliConfig {
    /// Describes settings related to the RPC endpoint
    pub rpc: RpcConfig,
    /// Describes settings related to the store.
    pub store: SqliteStoreConfig,
    /// Address of the Miden node to connect to.
    pub default_account_id: Option<String>,
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

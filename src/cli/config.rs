use core::fmt::Debug;

use figment::{
    value::{Dict, Map},
    Metadata, Profile, Provider,
};
use miden_client::{config::ClientConfig, store::Store};
use serde::{Deserialize, Serialize};

// CLI CONFIG
// ================================================================================================

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(bound = "")]
pub struct CliConfig<S: Store> {
    /// Configuration of the underlying client
    #[serde(flatten)]
    pub client_config: ClientConfig<S>,
    /// Address of the Miden node to connect to.
    pub default_account_id: Option<String>,
}

impl<S: Store> Default for CliConfig<S> {
    fn default() -> Self {
        Self {
            client_config: ClientConfig::<S>::default(),
            default_account_id: None,
        }
    }
}

// Make `ClientConfig` a provider itself for composability.
impl<S> Provider for CliConfig<S>
where
    S: Store,
    CliConfig<S>: Serialize,
{
    fn metadata(&self) -> Metadata {
        Metadata::named("CLI Config")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        figment::providers::Serialized::defaults(CliConfig::<S>::default()).data()
    }

    fn profile(&self) -> Option<Profile> {
        // Optionally, a profile that's selected by default.
        None
    }
}

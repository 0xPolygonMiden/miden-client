use std::{fs::File, io::Write, path::PathBuf, str::FromStr};

use clap::Parser;
use miden_client::rpc::Endpoint;

use crate::{
    config::{CliConfig, CliEndpoint},
    errors::CliError,
    CLIENT_CONFIG_FILE_NAME,
};

// Init COMMAND
// ================================================================================================

#[derive(Debug, Clone)]
enum Network {
    Custom(String),
    Devnet,
    Localhost,
    Testnet,
}

impl FromStr for Network {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "devnet" => Ok(Network::Devnet),
            "localhost" => Ok(Network::Localhost),
            "testnet" => Ok(Network::Testnet),
            custom => Ok(Network::Custom(custom.to_string())),
        }
    }
}

impl Network {
    /// Converts the Network variant to its corresponding RPC endpoint string
    pub fn to_rpc_endpoint(&self) -> String {
        match self {
            Network::Custom(custom) => custom.clone(),
            Network::Devnet => "https://rpc.devnet.miden.io".to_string(),
            Network::Localhost => Endpoint::default().to_string(),
            Network::Testnet => "https://rpc.testnet.miden.io".to_string(),
        }
    }
}

#[derive(Debug, Clone, Parser)]
#[clap(
    about = "Initialize the client. It will create a file named `miden-client.toml` that holds \
the CLI and client configurations, and will be placed by default in the current working \
directory."
)]
pub struct InitCmd {
    /// Network configuration to use. Options are `devnet`, `testnet`, `localhost` or a custom RPC
    /// endpoint. Defaults to the testnet network.
    #[clap(long, short, default_value = "testnet")]
    network: Option<Network>,

    /// Path to the store file.
    #[clap(long)]
    store_path: Option<String>,

    /// RPC endpoint for the proving service. Required if proving mode is set to remote.
    /// The endpoint must be in the form of "{protocol}://{hostname}:{port}", being the protocol
    /// and port optional.
    /// If the proving RPC isn't set, the proving mode will be set to local.
    #[clap(long)]
    remote_prover_endpoint: Option<String>,
}

impl InitCmd {
    pub fn execute(&self, config_file_path: PathBuf) -> Result<(), CliError> {
        if config_file_path.exists() {
            return Err(CliError::Config(
                "Error with the configuration file".to_string().into(),
                format!(
                    "The file \"{}\" already exists in the working directory. Please try using another directory or removing the file.",
                    CLIENT_CONFIG_FILE_NAME
                ),
            ));
        }

        let mut cli_config = CliConfig::default();

        if let Some(endpoint) = &self.network {
            let endpoint =
                CliEndpoint::try_from(endpoint.to_rpc_endpoint().as_str()).map_err(|err| {
                    CliError::Parse(err.into(), "Failed to parse RPC endpoint".to_string())
                })?;

            cli_config.rpc.endpoint = endpoint;
        }

        if let Some(path) = &self.store_path {
            cli_config.store_filepath = PathBuf::from(path);
        }

        cli_config.remote_prover_endpoint = match &self.remote_prover_endpoint {
            Some(rpc) => CliEndpoint::try_from(rpc.as_str()).ok(),
            None => None,
        };

        let config_as_toml_string = toml::to_string_pretty(&cli_config).map_err(|err| {
            CliError::Config("failed to serialize config".to_string().into(), err.to_string())
        })?;

        let mut file_handle = File::options()
            .write(true)
            .create_new(true)
            .open(&config_file_path)
            .map_err(|err| {
                CliError::Config("failed to create config file".to_string().into(), err.to_string())
            })?;

        file_handle.write(config_as_toml_string.as_bytes()).map_err(|err| {
            CliError::Config("failed to write config file".to_string().into(), err.to_string())
        })?;

        println!("Config file successfully created at: {:?}", config_file_path);

        Ok(())
    }
}

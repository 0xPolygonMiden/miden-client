use std::{fs::File, io::Write, path::PathBuf, str::FromStr};

use clap::Parser;

use crate::{
    config::{CliConfig, CliEndpoint},
    CLIENT_CONFIG_FILE_NAME,
};

// Init COMMAND
// ================================================================================================

#[derive(Debug, Clone)]
enum Network {
    Devnet,
    Testnet,
    Custom(String),
}

impl FromStr for Network {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "devnet" => Ok(Network::Devnet),
            "testnet" => Ok(Network::Testnet),
            custom => Ok(Network::Custom(custom.to_string())),
        }
    }
}

impl Network {
    /// Converts the Network variant to its corresponding RPC endpoint string
    pub fn to_rpc_endpoint(&self) -> &str {
        match self {
            Network::Devnet => "https://rpc.devnet.miden.io",
            Network::Testnet => "https://rpc.testnet.miden.io",
            Network::Custom(custom) => custom,
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
    /// Network configuration to use. Options are devnet, testnet, or a custom RPC endpoint.
    /// Defaults to a local network.
    #[clap(long, short, value_enum)]
    network: Option<Network>,

    /// Store file path.
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
    pub fn execute(&self, config_file_path: PathBuf) -> Result<(), String> {
        if config_file_path.exists() {
            return Err(format!(
                "A filed named \"{}\" already exists in the working directory. Please try using another directory or removing the file.",
                CLIENT_CONFIG_FILE_NAME
            )
            .to_string());
        }

        let mut cli_config = CliConfig::default();

        if let Some(endpoint) = &self.network {
            let endpoint =
                CliEndpoint::try_from(endpoint.to_rpc_endpoint()).map_err(|err| err.to_string())?;

            cli_config.rpc.endpoint = endpoint;
        }

        if let Some(path) = &self.store_path {
            cli_config.store_filepath = PathBuf::from(path);
        }

        cli_config.remote_prover_endpoint = match &self.remote_prover_endpoint {
            Some(rpc) => CliEndpoint::try_from(rpc.as_str()).ok(),
            None => None,
        };

        let config_as_toml_string = toml::to_string_pretty(&cli_config)
            .map_err(|err| format!("Error formatting config: {err}"))?;

        let mut file_handle = File::options()
            .write(true)
            .create_new(true)
            .open(&config_file_path)
            .map_err(|err| format!("Error opening the file: {err}"))?;

        file_handle
            .write(config_as_toml_string.as_bytes())
            .map_err(|err| format!("Error writing to file: {err}"))?;

        println!("Config file successfully created at: {:?}", config_file_path);

        Ok(())
    }
}

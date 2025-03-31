use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    str::FromStr,
};

use crate::commands::init::BlockDelta::*;
use clap::Parser;
use miden_client::rpc::Endpoint;
use tracing::info;

use crate::{
    config::{CliConfig, CliEndpoint},
    errors::CliError,
    CLIENT_CONFIG_FILE_NAME,
};

/// Contains the account component template file generated on build.rs, corresponding to the
/// fungible faucet component.
const FAUCET_TEMPLATE_FILE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/templates/", "basic-fungible-faucet.mct"));

/// Contains the account component template file generated on build.rs, corresponding to the basic
/// auth component.
const BASIC_AUTH_TEMPLATE_FILE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/templates/", "basic-auth.mct"));

// INIT COMMAND
// ================================================================================================

#[derive(Debug, Clone)]
pub enum BlockDelta {
    Delta(u32),
    //NOTE: I am not a huge fan of "Any" as a name.
    Any,
}

impl FromStr for BlockDelta {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(BlockDelta::Any),
            custom => Ok(BlockDelta::Delta(custom.parse().map_err(|_| {
                CliError::InvalidArgument(
                    "Could not parse number of max-blocks correctly
Valid options are 'none' or any positive integer"
                        .to_string(),
                )
            })?)),
        }
    }
}

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
            Network::Devnet => Endpoint::devnet().to_string(),
            Network::Localhost => Endpoint::default().to_string(),
            Network::Testnet => Endpoint::testnet().to_string(),
        }
    }
}

#[derive(Debug, Clone, Parser)]
#[clap(
    about = "Initialize the client. It will create a file named `miden-client.toml` that holds \
the CLI and client configurations, and will be placed by default in the current working \
directory"
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

    /// TODO: Document
    #[clap(long)]
    block_delta: Option<BlockDelta>,
}

impl InitCmd {
    pub fn execute(&self, config_file_path: &PathBuf) -> Result<(), CliError> {
        if config_file_path.exists() {
            return Err(CliError::Config(
                "Error with the configuration file".to_string().into(),
                format!(
                    "The file \"{CLIENT_CONFIG_FILE_NAME}\" already exists in the working directory. Please try using another directory or removing the file.",
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

        if let Some(block_delta) = &self.block_delta {
            cli_config.max_block_number_delta = match block_delta {
                Delta(max) => Some(*max),
                Any => None,
            }
        } else {
            // If the user did not specify wether a block limit is needed, then the default is set.
            // WARNING: This is already done in CliConfig::default(), I only added it here for clarity. Remove comment/code later?
            cli_config.max_block_number_delta = Some(256);
        };

        let config_as_toml_string = toml::to_string_pretty(&cli_config).map_err(|err| {
            CliError::Config("failed to serialize config".to_string().into(), err.to_string())
        })?;

        let mut file_handle = File::options()
            .write(true)
            .create_new(true)
            .open(config_file_path)
            .map_err(|err| {
            CliError::Config("failed to create config file".to_string().into(), err.to_string())
        })?;

        write_template_files(&cli_config)?;

        file_handle.write(config_as_toml_string.as_bytes()).map_err(|err| {
            CliError::Config("failed to write config file".to_string().into(), err.to_string())
        })?;

        println!("Config file successfully created at: {}", config_file_path.display());

        Ok(())
    }
}

/// Creates the directory specified by `cli_config.component_template_directory`
/// and writes the default included component templates.
fn write_template_files(cli_config: &CliConfig) -> Result<(), CliError> {
    fs::create_dir_all(&cli_config.component_template_directory).map_err(|err| {
        CliError::Config(
            Box::new(err),
            "failed to create account component templates directory".into(),
        )
    })?;

    // Write the faucet template file.
    // TODO: io errors should probably have their own context.
    let faucet_template_path =
        cli_config.component_template_directory.join("basic-fungible-faucet.mct");
    let mut faucet_file = File::create(&faucet_template_path)?;
    faucet_file.write_all(FAUCET_TEMPLATE_FILE)?;

    let basic_auth_template_path = cli_config.component_template_directory.join("basic-auth.mct");
    let mut basic_auth_file = File::create(&basic_auth_template_path)?;
    basic_auth_file.write_all(BASIC_AUTH_TEMPLATE_FILE)?;

    info!(
        "Template files successfully created in: {:?}",
        cli_config.component_template_directory
    );

    Ok(())
}

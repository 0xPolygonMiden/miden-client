use std::{fs::File, io::Write, path::PathBuf};

use clap::Parser;
use miden_client::config::Endpoint;

use crate::{config::CliConfig, ProvingMode, CLIENT_CONFIG_FILE_NAME};

// Init COMMAND
// ================================================================================================

#[derive(Debug, Clone, Parser)]
#[clap(about = "Initialize the client")]
pub struct InitCmd {
    /// Rpc config in the form of "{protocol}://{hostname}:{port}", being the protocol and port
    /// optional. If not provided user will be
    /// asked for input
    #[clap(long)]
    rpc: Option<String>,

    /// Store file path
    #[clap(long)]
    store_path: Option<String>,

    /// Proving mode. It can be performed locally or remotely. If the remote option is selected,
    /// the RPC endpoint must be provided.
    #[clap(long, default_value = "local")]
    proving_mode: ProvingMode,

    /// RPC endpoint for the proving service. Required if proving mode is set to remote.
    /// The endpoint must be in the form of "{protocol}://{hostname}:{port}", being the protocol
    /// and port optional.
    #[clap(long)]
    proving_rpc: Option<String>,
}

impl InitCmd {
    pub fn execute(&self, config_file_path: PathBuf) -> Result<(), String> {
        if config_file_path.exists() {
            return Err(format!(
                "The file \"{}\" already exists in the working directory.",
                CLIENT_CONFIG_FILE_NAME
            )
            .to_string());
        }

        let mut cli_config = CliConfig::default();

        if let Some(endpoint) = &self.rpc {
            let endpoint = Endpoint::try_from(endpoint.as_str()).map_err(|err| err.to_string())?;

            cli_config.rpc.endpoint = endpoint;
        }

        if let Some(path) = &self.store_path {
            cli_config.store.database_filepath = path.to_string();
        }

        cli_config.proving_mode = self.proving_mode.clone();

        if self.proving_mode == ProvingMode::Remote && self.proving_rpc.is_none() {
            return Err("Proving mode is set to remote, but proving RPC endpoint is not provided."
                .to_string());
        }

        if let Some(proving_rpc) = &self.proving_rpc {
            let endpoint = Endpoint::try_from(proving_rpc.as_str())
                .map_err(|err| format!("Error parsing proving RPC endpoint: {err}"))?;

            cli_config.proving_rpc_endpoint = Some(endpoint);
        }

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

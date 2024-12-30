use std::{fs::File, io::Write, path::PathBuf};

use clap::Parser;

use crate::{
    config::{CliConfig, CliEndpoint},
    CLIENT_CONFIG_FILE_NAME,
};

// Init COMMAND
// ================================================================================================

#[derive(Debug, Clone, Parser)]
#[clap(about = "Initialize the client")]
pub struct InitCmd {
    /// Rpc config in the form of "{protocol}://{hostname}:{port}", being the protocol and port
    /// optional. If not provided user will be asked for input.
    #[clap(long)]
    rpc: Option<String>,

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
                "The file \"{}\" already exists in the working directory.",
                CLIENT_CONFIG_FILE_NAME
            )
            .to_string());
        }

        let mut cli_config = CliConfig::default();

        if let Some(endpoint) = &self.rpc {
            let endpoint =
                CliEndpoint::try_from(endpoint.as_str()).map_err(|err| err.to_string())?;

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

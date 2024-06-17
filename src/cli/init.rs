use std::{fs::File, io::Write, path::PathBuf};

use clap::Parser;
use miden_client::config::Endpoint;

use crate::cli::config::CliConfig;

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
}

impl InitCmd {
    pub fn execute(&self, config_file_path: PathBuf) -> Result<(), String> {
        let mut cli_config = CliConfig::default();
        if let Some(endpoint) = &self.rpc {
            let endpoint = Endpoint::try_from(endpoint.as_str()).map_err(|err| err.to_string())?;

            cli_config.rpc.endpoint = endpoint;
        }

        if let Some(path) = &self.store_path {
            cli_config.store.database_filepath = path.to_string();
        }

        let config_as_toml_string = toml::to_string_pretty(&cli_config)
            .map_err(|err| format!("error formatting config: {err}"))?;

        let mut file_handle = File::options()
            .write(true)
            .create_new(true)
            .open(&config_file_path)
            .map_err(|err| format!("error opening the file: {err}"))?;
        file_handle
            .write(config_as_toml_string.as_bytes())
            .map_err(|err| format!("error writing to file: {err}"))?;

        println!("Config file successfully created at: {:?}", config_file_path);

        Ok(())
    }
}

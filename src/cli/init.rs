use std::{
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

use clap::Parser;
use miden_client::config::{ClientConfig, Endpoint};

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

    /// Store file path. If not provided user will be
    /// asked for input
    #[clap(long)]
    store_path: Option<String>,
}

impl InitCmd {
    pub fn execute(&self, config_file_path: PathBuf) -> Result<(), String> {
        let mut client_config = ClientConfig::default();
        if let Some(endpoint) = &self.rpc {
            let endpoint = Endpoint::try_from(endpoint.as_str()).map_err(|err| err.to_string())?;

            client_config.rpc.endpoint = endpoint;
        }

        if let Some(path) = &self.store_path {
            client_config.store.database_filepath = path.to_string();
        } else {
            println!("Please provide the store file path:");
            let mut store_path: String = String::new();
            io::stdin().read_line(&mut store_path).expect("Should read line");
            if !Path::new(&store_path.trim()).exists() {
                return Err("The provided path does not exist".to_string());
            }
            client_config.store.database_filepath = store_path.trim().to_string();
        }

        let config_as_toml_string = toml::to_string_pretty(&client_config)
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

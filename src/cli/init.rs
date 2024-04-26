use std::{
    fs::File,
    io::{self, Write},
    path::PathBuf,
};

use clap::Parser;
use miden_client::config::{ClientConfig, Endpoint};

// Init COMMAND
// ================================================================================================

#[derive(Debug, Clone, Parser)]
#[clap(about = "Initialize the client")]
pub struct InitCmd {
    /// Use the Client's default configuration. Useful for non-interactive environments.
    #[clap(long, default_value_t = false, conflicts_with = "testnet")]
    default: bool,
    /// Configure the Client to use testnet. Useful for non-interactive environments.
    #[clap(long, default_value_t = false, conflicts_with = "default")]
    testnet: bool,
    /// Rpc config in the form of "{hostname}:{port}" or "{hostname}". Unless `--default` is
    /// provided, the user is still asked for input to configure the store
    #[clap(long, group = "rpc", conflicts_with = "testnet")]
    rpc: Option<String>,
}

impl InitCmd {
    pub fn execute(&self, config_file_path: PathBuf) -> Result<(), String> {
        let client_config = match (self.default, self.testnet, &self.rpc) {
            // No flags provided, full interactive
            (false, false, None) => {
                let mut client_config = ClientConfig::default();

                interactive_rpc_config(&mut client_config)?;
                interactive_store_config(&mut client_config)?;

                client_config
            },
            // Default flag provided
            (true, false, None) => ClientConfig::default(),
            // Testnet flag provided
            (false, true, None) => ClientConfig::testnet(),
            // Only rpc flag provided, input is still asked for store config
            (false, false, Some(endpoint)) => {
                let mut client_config = ClientConfig::default();
                let endpoint =
                    Endpoint::try_from(endpoint.as_str()).map_err(|err| err.to_string())?;

                client_config.rpc.endpoint = endpoint;

                interactive_store_config(&mut client_config)?;

                client_config
            },
            // Both default and rpc flags were provided, will use default for store config and the
            // provided rpc config
            (true, false, Some(endpoint)) => {
                let mut client_config = ClientConfig::default();
                client_config.rpc.endpoint = Endpoint::try_from(endpoint.as_str())?;

                client_config
            },
            _ => {
                panic!("should not be possible to enter here");
            },
        };

        let config_as_toml_string = toml::to_string_pretty(&client_config)
            .map_err(|err| format!("error formatting config: {err}"))?;

        println!("Creating config file at: {:?}", config_file_path);
        let mut file_handle = File::options()
            .write(true)
            .create_new(true)
            .open(config_file_path)
            .map_err(|err| format!("error opening the file: {err}"))?;
        file_handle
            .write(config_as_toml_string.as_bytes())
            .map_err(|err| format!("error writing to file: {err}"))?;

        Ok(())
    }
}

fn interactive_rpc_config(client_config: &mut ClientConfig) -> Result<(), String> {
    println!("Protocol (default: http):");
    let mut protocol: String = String::new();
    io::stdin().read_line(&mut protocol).expect("Should read line");
    protocol = protocol.trim().to_string();
    if protocol.is_empty() {
        protocol = client_config.rpc.endpoint.protocol().to_string();
    }

    println!("Host (default: localhost):");
    let mut host: String = String::new();
    io::stdin().read_line(&mut host).expect("Should read line");
    host = host.trim().to_string();
    if host.is_empty() {
        host = client_config.rpc.endpoint.host().to_string();
    }

    println!("Node RPC Port (default: 57291):");
    let mut port_str: String = String::new();
    io::stdin().read_line(&mut port_str).expect("Should read line");
    port_str = port_str.trim().to_string();
    let port: u16 = if !port_str.is_empty() {
        port_str.parse().map_err(|err| format!("Error parsing port: {err}"))?
    } else {
        client_config.rpc.endpoint.port()
    };

    client_config.rpc.endpoint = Endpoint::new(protocol, host, port);

    Ok(())
}

fn interactive_store_config(client_config: &mut ClientConfig) -> Result<(), String> {
    println!("Sqlite file path (default: ./store.sqlite3):");
    let mut database_filepath: String = String::new();
    io::stdin().read_line(&mut database_filepath).expect("Should read line");
    database_filepath = database_filepath.trim().to_string();
    if !database_filepath.is_empty() {
        client_config.store.database_filepath = database_filepath;
    }

    Ok(())
}

use std::{
    fs::File,
    io::{self, Write},
    path::PathBuf,
};

use miden_client::config::ClientConfig;

pub(crate) fn initialize_client(config_file_path: PathBuf) -> Result<(), String> {
    let mut client_config = ClientConfig::default();

    println!("Protocol (default: http):");
    let mut protocol: String = String::new();
    io::stdin().read_line(&mut protocol).expect("Should read line");
    protocol = protocol.trim().to_string();
    if !protocol.is_empty() {
        client_config.rpc.endpoint.protocol = protocol;
    }

    println!("Host (default: localhost):");
    let mut host: String = String::new();
    io::stdin().read_line(&mut host).expect("Should read line");
    host = host.trim().to_string();
    if !host.is_empty() {
        client_config.rpc.endpoint.host = host;
    }

    println!("Node RPC Port (default: 57291):");
    let mut port_str: String = String::new();
    io::stdin().read_line(&mut port_str).expect("Should read line");
    port_str = port_str.trim().to_string();
    if !port_str.is_empty() {
        let port: u16 = port_str.parse().map_err(|err| format!("Error parsing port: {err}"))?;
        client_config.rpc.endpoint.port = port;
    }

    println!("Sqlite file path (default: ./store.sqlite3):");
    let mut database_filepath: String = String::new();
    io::stdin().read_line(&mut database_filepath).expect("Should read line");
    database_filepath = database_filepath.trim().to_string();
    if !database_filepath.is_empty() {
        client_config.store.database_filepath = database_filepath;
    }

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

use std::fs;

use miden_client::{
    auth::TransactionAuthenticator,
    crypto::FeltRng,
    rpc::NodeRpcClient,
    store::{NoteFilter, Store},
    Client,
};

use super::config::CliConfig;

pub fn print_client_info(client: &Client, config: &CliConfig) -> Result<(), String> {
    println!("Client version: {}", env!("CARGO_PKG_VERSION"));
    print_config_stats(config)?;
    print_client_stats(client)
}

// HELPERS
// ================================================================================================
fn print_client_stats(client: &Client) -> Result<(), String> {
    println!("Block number: {}", client.get_sync_height().map_err(|e| e.to_string())?);
    println!(
        "Tracked accounts: {}",
        client.get_account_headers().map_err(|e| e.to_string())?.len()
    );
    println!(
        "Expected notes: {}",
        client.get_input_notes(NoteFilter::Expected).map_err(|e| e.to_string())?.len()
    );
    Ok(())
}

fn print_config_stats(config: &CliConfig) -> Result<(), String> {
    println!("Node address: {}", config.rpc.endpoint.host());
    let store_len = fs::metadata(config.store.database_filepath.clone())
        .map_err(|e| e.to_string())?
        .len();
    println!("Store size: {} kB", store_len / 1024);
    println!(
        "Default account: {}",
        config.default_account_id.as_ref().unwrap_or(&"-".to_string())
    );
    Ok(())
}

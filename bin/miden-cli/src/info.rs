use std::fs;

use miden_client::{Client, store::NoteFilter};

use super::config::CliConfig;
use crate::{errors::CliError, load_config_file};

pub async fn print_client_info(client: &Client) -> Result<(), CliError> {
    let (config, _) = load_config_file()?;

    println!("Client version: {}", env!("CARGO_PKG_VERSION"));
    print_config_stats(&config)?;
    print_client_stats(client).await
}

// HELPERS
// ================================================================================================
async fn print_client_stats(client: &Client) -> Result<(), CliError> {
    println!("Block number: {}", client.get_sync_height().await?);
    println!("Tracked accounts: {}", client.get_account_headers().await?.len());
    println!("Expected notes: {}", client.get_input_notes(NoteFilter::Expected).await?.len());
    Ok(())
}

fn print_config_stats(config: &CliConfig) -> Result<(), CliError> {
    println!("Node address: {}", config.rpc.endpoint.0.host());
    let store_len = fs::metadata(config.store_filepath.clone())?.len();
    println!("Store size: {} kB", store_len / 1024);
    println!(
        "Default account: {}",
        config.default_account_id.as_ref().unwrap_or(&"-".to_string())
    );
    Ok(())
}

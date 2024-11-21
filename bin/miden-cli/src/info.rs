use std::fs;

use miden_client::{crypto::FeltRng, store::NoteFilter, Client};

use super::config::CliConfig;

pub async fn print_client_info(
    client: &Client<impl FeltRng>,
    config: &CliConfig,
) -> Result<(), String> {
    println!("Client version: {}", env!("CARGO_PKG_VERSION"));
    print_config_stats(config)?;
    print_client_stats(client).await
}

// HELPERS
// ================================================================================================
async fn print_client_stats(client: &Client<impl FeltRng>) -> Result<(), String> {
    println!("Block number: {}", client.get_sync_height().await.map_err(|e| e.to_string())?);
    println!(
        "Tracked accounts: {}",
        client.get_account_headers().await.map_err(|e| e.to_string())?.len()
    );
    println!(
        "Expected notes: {}",
        client
            .get_input_notes(NoteFilter::Expected)
            .await
            .map_err(|e| e.to_string())?
            .len()
    );
    Ok(())
}

fn print_config_stats(config: &CliConfig) -> Result<(), String> {
    println!("Node address: {}", config.rpc.endpoint.0.host);
    let store_len = fs::metadata(config.store_filepath.clone()).map_err(|e| e.to_string())?.len();
    println!("Store size: {} kB", store_len / 1024);
    println!(
        "Default account: {}",
        config.default_account_id.as_ref().unwrap_or(&"-".to_string())
    );
    Ok(())
}

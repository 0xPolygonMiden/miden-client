use std::fs;

use miden_client::{
    client::{rpc::NodeRpcClient, Client},
    config::ClientConfig,
    store::{NoteFilter, Store},
};
use miden_objects::crypto::rand::FeltRng;

pub fn print_client_info<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &Client<N, R, S>,
    config: &ClientConfig,
) -> Result<(), String> {
    println!("client version: {}", env!("CARGO_PKG_VERSION"));
    print_config_stats(config)?;
    print_client_stats(client)
}

// HELPERS
// ================================================================================================
fn print_client_stats<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &Client<N, R, S>,
) -> Result<(), String> {
    println!("block number: {}", client.get_sync_height().map_err(|e| e.to_string())?);
    println!(
        "tracked accounts: {}",
        client.get_account_stubs().map_err(|e| e.to_string())?.len()
    );
    println!(
        "pending notes: {}",
        client.get_input_notes(NoteFilter::Pending).map_err(|e| e.to_string())?.len()
    );
    Ok(())
}

fn print_config_stats(config: &ClientConfig) -> Result<(), String> {
    println!("node address: {}", config.rpc.endpoint.host());
    let store_len = fs::metadata(config.store.database_filepath.clone())
        .map_err(|e| e.to_string())?
        .len();
    println!("store size: {} kB", store_len / 1024);
    println!(
        "default account: {}",
        config
            .cli
            .as_ref()
            .and_then(|cli| cli.default_account_id.as_ref())
            .unwrap_or(&"-".to_string())
    );
    Ok(())
}

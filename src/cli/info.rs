use std::fs;

use miden_client::{
    rpc::NodeRpcClient,
    store::{sqlite_store::SqliteStore, NoteFilter, Store},
    Client,
};
use miden_objects::crypto::rand::FeltRng;
use miden_tx::auth::TransactionAuthenticator;

use super::config::CliConfig;

pub fn print_client_info<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: &Client<N, R, S, A>,
    config: &CliConfig<SqliteStore>,
) -> Result<(), String> {
    println!("Client version: {}", env!("CARGO_PKG_VERSION"));
    print_config_stats(config)?;
    print_client_stats(client)
}

// HELPERS
// ================================================================================================
fn print_client_stats<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: &Client<N, R, S, A>,
) -> Result<(), String> {
    println!("Block number: {}", client.get_sync_height().map_err(|e| e.to_string())?);
    println!(
        "Tracked accounts: {}",
        client.get_account_stubs().map_err(|e| e.to_string())?.len()
    );
    println!(
        "Pending notes: {}",
        client.get_input_notes(NoteFilter::Pending).map_err(|e| e.to_string())?.len()
    );
    Ok(())
}

fn print_config_stats(config: &CliConfig<SqliteStore>) -> Result<(), String> {
    println!("Node address: {}", config.client_config.rpc.endpoint.host());
    let store_len = fs::metadata(config.client_config.store.database_filepath.clone())
        .map_err(|e| e.to_string())?
        .len();
    println!("Store size: {} kB", store_len / 1024);
    println!(
        "Default account: {}",
        config.default_account_id.as_ref().unwrap_or(&"-".to_string())
    );
    Ok(())
}

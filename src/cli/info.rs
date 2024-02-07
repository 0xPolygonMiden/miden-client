use miden_client::client::{Client, NodeApi};
use miden_tx::DataStore;

pub fn print_client_info<N: NodeApi, D: DataStore>(client: &Client<N, D>) -> Result<(), String> {
    print_block_number(client)
}

// HELPERS
// ================================================================================================
fn print_block_number<N: NodeApi, D: DataStore>(client: &Client<N, D>) -> Result<(), String> {
    println!(
        "block number: {}",
        client.get_sync_height().map_err(|e| e.to_string())?
    );
    Ok(())
}

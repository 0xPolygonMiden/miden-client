use miden_client::client::{Client, NodeApi};

pub fn print_client_info<N: NodeApi>(client: &Client<N>) -> Result<(), String> {
    print_block_number(client)
}

// HELPERS
// ================================================================================================
fn print_block_number<N: NodeApi>(client: &Client<N>) -> Result<(), String> {
    println!(
        "block number: {}",
        client.get_sync_height().map_err(|e| e.to_string())?
    );
    Ok(())
}

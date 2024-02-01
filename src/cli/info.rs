use miden_client::client::Client;

pub fn print_client_info(client: &Client) -> Result<(), String> {
    print_block_number(client)
}

// HELPERS
// ================================================================================================
fn print_block_number(client: &Client) -> Result<(), String> {
    println!(
        "block number: {}",
        client.get_sync_height().map_err(|e| e.to_string())?
    );
    Ok(())
}

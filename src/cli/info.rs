use miden_client::{
    client::{rpc::NodeRpcClient, Client},
    store::Store,
};

pub fn print_client_info<N: NodeRpcClient, S: Store, E: Store>(
    client: &Client<N, S, E>
) -> Result<(), String> {
    print_block_number(client)
}

// HELPERS
// ================================================================================================
fn print_block_number<N: NodeRpcClient, S: Store, E: Store>(
    client: &Client<N, S, E>
) -> Result<(), String> {
    println!("block number: {}", client.get_sync_height().map_err(|e| e.to_string())?);
    Ok(())
}

use miden_client::{
    client::{rpc::NodeRpcClient, Client},
    store::Store,
};
use miden_objects::crypto::rand::FeltRng;

pub fn print_client_info<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &Client<N, R, S>,
) -> Result<(), String> {
    print_block_number(client)
}

// HELPERS
// ================================================================================================

fn print_block_number<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &Client<N, R, S>,
) -> Result<(), String> {
    println!("block number: {}", client.get_sync_height().map_err(|e| e.to_string())?);
    Ok(())
}

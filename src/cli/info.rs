use miden_client::{
    client::{rpc::NodeRpcClient, Client},
    store::Store,
};
use miden_tx::DataStore;

pub fn print_client_info<N: NodeRpcClient, S: Store, D: DataStore>(
    client: &Client<N, S, D>,
) -> Result<(), String> {
    print_block_number(client)
}

// HELPERS
// ================================================================================================
fn print_block_number<N: NodeRpcClient, S: Store, D: DataStore>(
    client: &Client<N, S, D>,
) -> Result<(), String> {
    println!(
        "block number: {}",
        client.get_sync_height().map_err(|e| e.to_string())?
    );
    Ok(())
}

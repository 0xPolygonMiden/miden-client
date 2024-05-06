use miden_client::{
    client::{rpc::NodeRpcClient, Client},
    store::Store,
};
use miden_objects::crypto::rand::FeltRng;
use miden_tx::TransactionAuthenticator;

pub fn print_client_info<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: &Client<N, R, S, A>,
) -> Result<(), String> {
    print_block_number(client)
}

// HELPERS
// ================================================================================================
fn print_block_number<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: &Client<N, R, S, A>,
) -> Result<(), String> {
    println!("block number: {}", client.get_sync_height().map_err(|e| e.to_string())?);
    Ok(())
}

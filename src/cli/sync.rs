use miden_client::{
    client::{rpc::NodeRpcClient, Client},
    store::Store,
};
use miden_objects::crypto::rand::FeltRng;
use miden_tx::TransactionAuthenticator;

pub async fn sync_state<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    mut client: Client<N, R, S, A>,
) -> Result<(), String> {
    let block_num = client.sync_state().await?;
    println!("State synced to block {}", block_num);
    Ok(())
}

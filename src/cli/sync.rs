use miden_client::client::{Client, NodeRpcClient};
use miden_tx::DataStore;

pub async fn sync_state<N: NodeRpcClient, D: DataStore>(
    mut client: Client<N, D>,
) -> Result<(), String> {
    let block_num = client.sync_state().await?;
    println!("State synced to block {}", block_num);
    Ok(())
}

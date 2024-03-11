use miden_client::{
    client::{rpc::NodeRpcClient, Client},
    store::Store,
};
use miden_tx::DataStore;

pub async fn sync_state<N: NodeRpcClient, S: Store, D: DataStore>(
    mut client: Client<N, S, D>,
) -> Result<(), String> {
    let block_num = client.sync_state().await?;
    println!("State synced to block {}", block_num);
    Ok(())
}

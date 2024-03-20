use miden_client::{
    client::{rpc::NodeRpcClient, Client},
    store::Store,
};

pub async fn sync_state<N: NodeRpcClient, S: Store, E: Store>(
    mut client: Client<N, S, E>
) -> Result<(), String> {
    let block_num = client.sync_state().await?;
    println!("State synced to block {}", block_num);
    Ok(())
}

use miden_client::client::{Client, NodeApi};

pub async fn sync_state<N: NodeApi>(mut client: Client<N>) -> Result<(), String> {
    let block_num = client.sync_state().await?;
    println!("State synced to block {}", block_num);
    Ok(())
}

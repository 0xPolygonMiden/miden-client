use miden_client::client::Client;

pub async fn sync_state(mut client: Client) -> Result<(), String> {
    let block_num = client.sync_state().await.map_err(|e| e.to_string())?;
    println!("state synced to block {}", block_num);
    Ok(())
}

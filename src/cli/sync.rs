use miden_client::client::Client;

pub async fn state_sync(mut client: Client) -> Result<(), String> {
    let block_num = client.state_sync().await.map_err(|e| e.to_string())?;
    println!("state synced to block {}", block_num);
    Ok(())
}

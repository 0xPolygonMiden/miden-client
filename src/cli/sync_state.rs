use crate::Client;
use clap::Parser;

// TAGS COMMAND
// ================================================================================================

#[derive(Debug, Clone, Parser)]
#[clap(about = "View and modify the state metadata associated with the client.")]
pub enum SyncStateCmd {
    /// Sync this client with the latest state of the Miden network.
    #[clap(short_flag = 's')]
    SyncState,

    /// List all tags monitored by this client
    #[clap(short_flag = 'l')]
    ListTags,

    /// Add a new tag to the list of tags monitored by this client
    #[clap(short_flag = 'a')]
    AddTag {
        #[clap()]
        tag: u64,
    },

    /// View the block number this client is synced to
    #[clap(short_flag = 'b')]
    BlockNumber,
}

impl SyncStateCmd {
    pub async fn execute(&self, client: Client) -> Result<(), String> {
        match self {
            SyncStateCmd::SyncState => {
                sync_state(client).await?;
            }
            SyncStateCmd::ListTags => {
                list_tags(client)?;
            }
            SyncStateCmd::AddTag { tag } => {
                add_tag(client, *tag)?;
            }
            SyncStateCmd::BlockNumber => {
                print_block_number(client)?;
            }
        }
        Ok(())
    }
}

// HELPERS
// ================================================================================================
fn list_tags(client: Client) -> Result<(), String> {
    let tags = client.get_note_tags().map_err(|err| err.to_string())?;
    println!("tags: {:?}", tags);
    Ok(())
}

fn add_tag(mut client: Client, tag: u64) -> Result<(), String> {
    client.add_note_tag(tag).map_err(|err| err.to_string())?;
    println!("tag {} added", tag);
    Ok(())
}

fn print_block_number(client: Client) -> Result<(), String> {
    println!(
        "block number: {}",
        client
            .get_latest_block_number()
            .map_err(|e| e.to_string())?
    );
    Ok(())
}

async fn sync_state(mut client: Client) -> Result<(), String> {
    let block_num = client.sync_state().await.map_err(|e| e.to_string())?;
    println!("state synced to block {}", block_num);
    Ok(())
}

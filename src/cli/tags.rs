use miden_client::client::NodeRpcClient;
use miden_tx::DataStore;

use super::{Client, Parser};

#[derive(Debug, Parser, Clone)]
#[clap(about = "View and add tags")]
pub enum TagsCmd {
    /// List all tags monitored by this client
    #[clap(short_flag = 'l')]
    List,

    /// Add a new tag to the list of tags monitored by this client
    #[clap(short_flag = 'a')]
    Add {
        #[clap()]
        tag: u64,
    },
}

impl TagsCmd {
    pub async fn execute<N: NodeRpcClient, D: DataStore>(
        &self,
        client: Client<N, D>,
    ) -> Result<(), String> {
        match self {
            TagsCmd::List => {
                list_tags(client)?;
            }
            TagsCmd::Add { tag } => {
                add_tag(client, *tag)?;
            }
        }
        Ok(())
    }
}

// HELPERS
// ================================================================================================
fn list_tags<N: NodeRpcClient, D: DataStore>(client: Client<N, D>) -> Result<(), String> {
    let tags = client.get_note_tags()?;
    println!("tags: {:?}", tags);
    Ok(())
}

fn add_tag<N: NodeRpcClient, D: DataStore>(
    mut client: Client<N, D>,
    tag: u64,
) -> Result<(), String> {
    client.add_note_tag(tag)?;
    println!("tag {} added", tag);
    Ok(())
}

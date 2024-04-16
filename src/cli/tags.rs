use miden_client::{client::rpc::NodeRpcClient, store::Store};
use miden_objects::crypto::rand::FeltRng;

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
        tag: u32,
    },
}

impl TagsCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store>(
        &self,
        client: Client<N, R, S>,
    ) -> Result<(), String> {
        match self {
            TagsCmd::List => {
                list_tags(client)?;
            },
            TagsCmd::Add { tag } => {
                add_tag(client, *tag)?;
            },
        }
        Ok(())
    }
}

// HELPERS
// ================================================================================================
fn list_tags<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: Client<N, R, S>,
) -> Result<(), String> {
    let tags = client.get_note_tags()?;
    println!("tags: {:?}", tags);
    Ok(())
}

fn add_tag<N: NodeRpcClient, R: FeltRng, S: Store>(
    mut client: Client<N, R, S>,
    tag: u32,
) -> Result<(), String> {
    client.add_note_tag(tag.into())?;
    println!("tag {} added", tag);
    Ok(())
}

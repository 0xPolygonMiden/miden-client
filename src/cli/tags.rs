use miden_client::{client::rpc::NodeRpcClient, store::Store};
use miden_objects::{
    crypto::rand::FeltRng,
    notes::{NoteExecutionHint, NoteTag},
};
use miden_tx::TransactionAuthenticator;
use tracing::info;

use super::{Client, Parser};

#[derive(Default, Debug, Parser, Clone)]
#[clap(about = "View and manage tags. Defaults to `list` command.")]
pub struct TagsCmd {
    /// List all tags monitored by this client
    #[clap(short, long, group = "action")]
    list: bool,

    /// Add a new tag to the list of tags monitored by this client
    #[clap(short, long, group = "action", value_name = "tag")]
    add: Option<u32>,

    /// Removes a tag from the list of tags monitored by this client
    #[clap(short, long, group = "action", value_name = "tag")]
    remove: Option<u32>,
}

impl TagsCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        match self {
            TagsCmd { add: Some(tag), .. } => {
                add_tag(client, *tag)?;
            },
            TagsCmd { remove: Some(tag), .. } => {
                remove_tag(client, *tag)?;
            },
            _ => {
                list_tags(client)?;
            },
        }
        Ok(())
    }
}

// HELPERS
// ================================================================================================
fn list_tags<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: Client<N, R, S, A>,
) -> Result<(), String> {
    let tags = client.get_note_tags()?;
    println!("Tags: {:?}", tags);
    Ok(())
}

fn add_tag<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    mut client: Client<N, R, S, A>,
    tag: u32,
) -> Result<(), String> {
    let tag: NoteTag = tag.into();
    let execution_mode = match tag.execution_hint() {
        NoteExecutionHint::Local => "Local",
        NoteExecutionHint::Network => "Network",
    };
    info!(
        "adding tag - Single Target? {} - Execution mode: {}",
        tag.is_single_target(),
        execution_mode
    );
    client.add_note_tag(tag)?;
    println!("Tag {} added", tag);
    Ok(())
}

fn remove_tag<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    mut client: Client<N, R, S, A>,
    tag: u32,
) -> Result<(), String> {
    client.remove_note_tag(tag.into())?;
    println!("Tag {} removed", tag);
    Ok(())
}

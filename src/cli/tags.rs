use miden_client::{client::rpc::NodeRpcClient, store::Store};
use miden_objects::{
    crypto::rand::FeltRng,
    notes::{NoteExecutionMode, NoteTag},
};
use tracing::info;

use super::{Client, Parser};

#[derive(Default, Debug, Parser, Clone)]
#[clap(about = "View and manage tags. Defaults to `list` command.")]
pub enum TagsCmd {
    /// List all tags monitored by this client
    #[default]
    #[clap(short_flag = 'l')]
    List,

    /// Add a new tag to the list of tags monitored by this client
    #[clap(short_flag = 'a')]
    Add {
        #[clap()]
        tag: u32,
    },

    /// Removes a tag from the list of tags monitored by this client
    #[clap(short_flag = 'r')]
    Remove {
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
            TagsCmd::Remove { tag } => {
                remove_tag(client, *tag)?;
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
    println!("Tags: {:?}", tags);
    Ok(())
}

fn add_tag<N: NodeRpcClient, R: FeltRng, S: Store>(
    mut client: Client<N, R, S>,
    tag: u32,
) -> Result<(), String> {
    let tag: NoteTag = tag.into();
    let execution_mode = match tag.execution_mode() {
        NoteExecutionMode::Local => "Local",
        NoteExecutionMode::Network => "Network",
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

fn remove_tag<N: NodeRpcClient, R: FeltRng, S: Store>(
    mut client: Client<N, R, S>,
    tag: u32,
) -> Result<(), String> {
    client.remove_note_tag(tag.into())?;
    println!("Tag {} removed", tag);
    Ok(())
}

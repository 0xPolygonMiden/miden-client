use miden_client::{
    crypto::FeltRng,
    notes::{NoteExecutionMode, NoteTag},
    Client,
};
use tracing::info;

use crate::Parser;

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
    pub async fn execute(&self, client: Client<impl FeltRng>) -> Result<(), String> {
        match self {
            TagsCmd { add: Some(tag), .. } => {
                add_tag(client, *tag).await?;
            },
            TagsCmd { remove: Some(tag), .. } => {
                remove_tag(client, *tag).await?;
            },
            _ => {
                list_tags(client).await?;
            },
        }
        Ok(())
    }
}

// HELPERS
// ================================================================================================
async fn list_tags(client: Client<impl FeltRng>) -> Result<(), String> {
    let tags = client.get_note_tags().await?;
    println!("Tags: {:?}", tags);
    Ok(())
}

async fn add_tag(mut client: Client<impl FeltRng>, tag: u32) -> Result<(), String> {
    let tag: NoteTag = tag.into();
    let execution_mode = match tag.execution_hint() {
        NoteExecutionMode::Local => "Local",
        NoteExecutionMode::Network => "Network",
    };
    info!(
        "adding tag - Single Target? {} - Execution mode: {}",
        tag.is_single_target(),
        execution_mode
    );
    client.add_note_tag(tag).await?;
    println!("Tag {} added", tag);
    Ok(())
}

async fn remove_tag(mut client: Client<impl FeltRng>, tag: u32) -> Result<(), String> {
    client.remove_note_tag(tag.into()).await?;
    println!("Tag {} removed", tag);
    Ok(())
}

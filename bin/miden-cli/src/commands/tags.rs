use miden_client::{
    Client,
    note::{NoteExecutionMode, NoteTag},
};
use tracing::info;

use crate::{Parser, errors::CliError};

#[derive(Default, Debug, Parser, Clone)]
#[command(about = "View and manage tags. Defaults to `list` command")]
pub struct TagsCmd {
    /// List all tags monitored by this client.
    #[arg(short, long, group = "action")]
    list: bool,

    /// Add a new tag to the list of tags monitored by this client.
    #[arg(short, long, group = "action", value_name = "tag")]
    add: Option<u32>,

    /// Removes a tag from the list of tags monitored by this client.
    #[arg(short, long, group = "action", value_name = "tag")]
    remove: Option<u32>,
}

impl TagsCmd {
    pub async fn execute(&self, client: Client) -> Result<(), CliError> {
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
async fn list_tags(client: Client) -> Result<(), CliError> {
    let tags = client.get_note_tags().await?;
    println!("Tags: {tags:?}");
    Ok(())
}

async fn add_tag(mut client: Client, tag: u32) -> Result<(), CliError> {
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
    client.add_note_tag(tag).await?;
    println!("Tag {tag} added");
    Ok(())
}

async fn remove_tag(mut client: Client, tag: u32) -> Result<(), CliError> {
    client.remove_note_tag(tag.into()).await?;
    println!("Tag {tag} removed");
    Ok(())
}

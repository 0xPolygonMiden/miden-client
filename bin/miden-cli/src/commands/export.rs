use std::{fs::File, io::Write, path::PathBuf};

use miden_client::{
    accounts::AccountData,
    auth::TransactionAuthenticator,
    crypto::FeltRng,
    notes::NoteFile,
    rpc::NodeRpcClient,
    store::{NoteStatus, Store},
    utils::Serializable,
    Client,
};
use tracing::info;

use crate::{get_output_note_with_id_prefix, utils::parse_account_id, Parser};

#[derive(Debug, Parser, Clone)]
#[clap(about = "Export client output notes")]
pub struct ExportCmd {
    /// ID (or a valid prefix) of the output note or account to export
    #[clap()]
    id: String,

    /// Desired filename for the binary file. Defaults to the note ID if not provided
    #[clap(short, long)]
    filename: Option<PathBuf>,

    /// Export account data (cannot be used with --note)
    #[arg(long, conflicts_with = "note")]
    account: bool,

    /// Export note data (cannot be used with --account)
    #[arg(long, requires = "export_type", conflicts_with = "account")]
    note: bool,

    /// Exported note type
    #[clap(short, long, value_enum, conflicts_with = "account")]
    export_type: Option<ExportType>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ExportType {
    Id,
    Full,
    Partial,
}

impl ExportCmd {
    pub fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        mut client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        if self.account {
            export_account(&client, self.id.as_str(), self.filename.clone())?;
        } else {
            export_note(
                &mut client,
                self.id.as_str(),
                self.filename.clone(),
                self.export_type.clone().expect("Note export must have an export type"),
            )?;
        }
        Ok(())
    }
}

// EXPORT ACCOUNT
// ================================================================================================

fn export_account<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: &Client<N, R, S, A>,
    account_id: &str,
    filename: Option<PathBuf>,
) -> Result<File, String> {
    let account_id = parse_account_id(client, account_id)?;

    let (account, account_seed) = client.get_account(account_id)?;

    let auth = client.get_account_auth(account_id)?;

    let account_data = AccountData::new(account, account_seed, auth);

    let file_path = if let Some(filename) = filename {
        filename
    } else {
        let current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
        current_dir.join(format!("{}.mac", account_id))
    };

    info!("Writing file to {}", file_path.to_string_lossy());
    let mut file = File::create(file_path).map_err(|err| err.to_string())?;
    account_data.write_into(&mut file);

    println!("Succesfully exported account {}", account_id);
    Ok(file)
}

// EXPORT NOTE
// ================================================================================================

fn export_note<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: &mut Client<N, R, S, A>,
    note_id: &str,
    filename: Option<PathBuf>,
    export_type: ExportType,
) -> Result<File, String> {
    let note_id = get_output_note_with_id_prefix(client, note_id)
        .map_err(|err| err.to_string())?
        .id();

    let output_note = client
        .get_output_notes(miden_client::store::NoteFilter::Unique(note_id))?
        .pop()
        .expect("should have an output note");

    let note_file = match export_type {
        ExportType::Id => NoteFile::NoteId(output_note.id()),
        ExportType::Full => match output_note.inclusion_proof() {
            Some(inclusion_proof) => {
                NoteFile::NoteWithProof(output_note.clone().try_into()?, inclusion_proof.clone())
            },
            None => return Err("Note does not have inclusion proof".to_string()),
        },
        ExportType::Partial => {
            let after_block_num = match output_note.status() {
                NoteStatus::Expected { block_height, .. } => block_height.unwrap_or(0),
                _ => {
                    output_note
                        .inclusion_proof()
                        .expect("Committed notes should have inclusion proof")
                        .location()
                        .block_num()
                        - 1
                },
            };

            NoteFile::NoteDetails {
                details: output_note.clone().try_into()?,
                after_block_num,
                tag: Some(output_note.metadata().tag()),
            }
        },
    };

    let file_path = if let Some(filename) = filename {
        filename
    } else {
        let current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
        current_dir.join(format!("{}.mno", note_id.inner()))
    };

    info!("Writing file to {}", file_path.to_string_lossy());
    let mut file = File::create(file_path).map_err(|err| err.to_string())?;
    file.write_all(&note_file.to_bytes()).map_err(|err| err.to_string())?;

    println!("Succesfully exported note {}", note_id);
    Ok(file)
}

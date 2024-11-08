use std::{fs::File, io::Write, path::PathBuf};

use miden_client::{
    accounts::AccountData, crypto::FeltRng, store::NoteExportType, utils::Serializable, Client,
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

impl From<ExportType> for NoteExportType {
    fn from(export_type: ExportType) -> NoteExportType {
        match export_type {
            ExportType::Id => NoteExportType::NoteId,
            ExportType::Full => NoteExportType::NoteWithProof,
            ExportType::Partial => NoteExportType::NoteDetails,
        }
    }
}

impl ExportCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), String> {
        if self.account {
            export_account(&client, self.id.as_str(), self.filename.clone()).await?;
        } else if let Some(export_type) = &self.export_type {
            export_note(&mut client, self.id.as_str(), self.filename.clone(), export_type.clone())
                .await?;
        } else {
            return Err("Export type is required when exporting a note".to_string());
        }
        Ok(())
    }
}

// EXPORT ACCOUNT
// ================================================================================================

async fn export_account<R: FeltRng>(
    client: &Client<R>,
    account_id: &str,
    filename: Option<PathBuf>,
) -> Result<File, String> {
    let account_id = parse_account_id(client, account_id).await?;

    let (account, account_seed) = client.get_account(account_id).await?;

    let auth = client.get_account_auth(account_id).await?;

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

async fn export_note(
    client: &mut Client<impl FeltRng>,
    note_id: &str,
    filename: Option<PathBuf>,
    export_type: ExportType,
) -> Result<File, String> {
    let note_id = get_output_note_with_id_prefix(client, note_id)
        .await
        .map_err(|err| err.to_string())?
        .id();

    let output_note = client
        .get_output_notes(miden_client::store::NoteFilter::Unique(note_id))
        .await?
        .pop()
        .expect("should have an output note");

    let note_file = output_note.into_note_file(export_type.into())?;

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

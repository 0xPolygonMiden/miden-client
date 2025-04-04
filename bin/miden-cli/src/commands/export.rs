use std::{fs::File, io::Write, path::PathBuf};

use miden_client::{
    Client, ClientError, Word,
    account::{Account, AccountFile},
    store::NoteExportType,
    transaction::AccountInterface,
    utils::Serializable,
};
use miden_lib::AuthScheme;
use miden_objects::AccountError;
use tracing::info;

use crate::{
    CliKeyStore, Parser, errors::CliError, get_output_note_with_id_prefix, utils::parse_account_id,
};

#[derive(Debug, Parser, Clone)]
#[clap(about = "Export client output notes, or account data")]
pub struct ExportCmd {
    /// ID (or a valid prefix) of the output note or account to export.
    #[clap()]
    id: String,

    /// Desired filename for the binary file. Defaults to the note ID if not provided.
    #[clap(short, long)]
    filename: Option<PathBuf>,

    /// Export account data (cannot be used with --note).
    #[arg(long, conflicts_with = "note")]
    account: bool,

    /// Export note data (cannot be used with --account).
    #[arg(long, requires = "export_type", conflicts_with = "account")]
    note: bool,

    /// Exported note type.
    #[clap(short, long, value_enum, conflicts_with = "account")]
    export_type: Option<ExportType>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ExportType {
    Id,
    Full,
    Partial,
}

impl From<&ExportType> for NoteExportType {
    fn from(export_type: &ExportType) -> NoteExportType {
        match export_type {
            ExportType::Id => NoteExportType::NoteId,
            ExportType::Full => NoteExportType::NoteWithProof,
            ExportType::Partial => NoteExportType::NoteDetails,
        }
    }
}

impl ExportCmd {
    pub async fn execute(&self, mut client: Client, keystore: CliKeyStore) -> Result<(), CliError> {
        if self.account {
            export_account(&client, &keystore, self.id.as_str(), self.filename.clone()).await?;
        } else if let Some(export_type) = &self.export_type {
            export_note(&mut client, self.id.as_str(), self.filename.clone(), export_type).await?;
        } else {
            return Err(CliError::Export(
                "Export type is required when exporting a note".to_string(),
            ));
        }
        Ok(())
    }
}

// EXPORT ACCOUNT
// ================================================================================================

async fn export_account(
    client: &Client,
    keystore: &CliKeyStore,
    account_id: &str,
    filename: Option<PathBuf>,
) -> Result<File, CliError> {
    let account_id = parse_account_id(client, account_id).await?;

    let account = client
        .get_account(account_id)
        .await?
        .ok_or(CliError::Export(format!("Account with ID {account_id} not found")))?;
    let account_seed = account.seed().copied();

    let account: Account = account.into();

    let auth = keystore
        .get_key(get_public_key_from_account(&account)?)
        .map_err(CliError::KeyStore)?
        .ok_or(CliError::Export("Auth not found for account".to_string()))?;

    let account_data = AccountFile::new(account, account_seed, auth);

    let file_path = if let Some(filename) = filename {
        filename
    } else {
        let current_dir = std::env::current_dir()?;
        current_dir.join(format!("{account_id}.mac"))
    };

    info!("Writing file to {}", file_path.to_string_lossy());
    let mut file = File::create(file_path)?;
    account_data.write_into(&mut file);

    println!("Succesfully exported account {account_id}");
    Ok(file)
}

// EXPORT NOTE
// ================================================================================================

async fn export_note(
    client: &mut Client,
    note_id: &str,
    filename: Option<PathBuf>,
    export_type: &ExportType,
) -> Result<File, CliError> {
    let note_id = get_output_note_with_id_prefix(client, note_id)
        .await
        .map_err(|err| CliError::Export(err.to_string()))?
        .id();

    let output_note = client
        .get_output_notes(miden_client::store::NoteFilter::Unique(note_id))
        .await?
        .pop()
        .expect("should have an output note");

    let note_file = output_note
        .into_note_file(&export_type.into())
        .map_err(|err| CliError::Export(err.to_string()))?;

    let file_path = if let Some(filename) = filename {
        filename
    } else {
        let current_dir = std::env::current_dir()?;
        current_dir.join(format!("{}.mno", note_id.inner()))
    };

    info!("Writing file to {}", file_path.to_string_lossy());
    let mut file = File::create(file_path)?;
    file.write_all(&note_file.to_bytes()).map_err(CliError::IO)?;

    println!("Succesfully exported note {note_id}");
    Ok(file)
}

/// Gets the public key from the storage of an account. This will only work if the account is
/// created by the CLI as it expects the account to have the `RpoFalcon512` authentication scheme.
pub fn get_public_key_from_account(account: &Account) -> Result<Word, ClientError> {
    let interface: AccountInterface = account.into();
    let auth = interface.auth().first().ok_or(ClientError::AccountError(
        AccountError::AssumptionViolated("Account should have an auth scheme".to_string()),
    ))?;

    match auth {
        AuthScheme::RpoFalcon512 { pub_key } => Ok(Word::from(*pub_key)),
    }
}

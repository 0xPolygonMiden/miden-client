use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use miden_client::{
    accounts::{AccountData, AccountId},
    crypto::FeltRng,
    notes::NoteFile,
    utils::Deserializable,
    Client,
};
use tracing::info;

use crate::{commands::account::maybe_set_default_account, utils::load_config_file, Parser};

#[derive(Debug, Parser, Clone)]
#[clap(about = "Import client objects such as accounts and notes")]
pub struct ImportCmd {
    /// Paths to the files that contains the account/note data
    #[arg()]
    filenames: Vec<PathBuf>,
}

impl ImportCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), String> {
        validate_paths(&self.filenames)?;
        let (mut current_config, _) = load_config_file()?;
        for filename in &self.filenames {
            let note_file = read_note_file(filename.clone());

            if let Ok(note_file) = note_file {
                let note_id = client.import_note(note_file).await.map_err(|err| err.to_string())?;
                println!("Succesfully imported note {}", note_id.inner());
            } else {
                let account_id = import_account(&mut client, filename)
                    .await
                    .map_err(|_| format!("Failed to parse file {}", filename.to_string_lossy()))?;
                println!("Succesfully imported account {}", account_id);

                if account_id.is_regular_account() {
                    maybe_set_default_account(&mut current_config, account_id)?;
                }
            }
        }
        Ok(())
    }
}

// IMPORT ACCOUNT
// ================================================================================================

async fn import_account(
    client: &mut Client<impl FeltRng>,
    filename: &PathBuf,
) -> Result<AccountId, String> {
    info!(
        "Attempting to import account data from {}...",
        fs::canonicalize(filename).map_err(|err| err.to_string())?.as_path().display()
    );
    let account_data_file_contents = fs::read(filename).map_err(|err| err.to_string())?;
    let account_data =
        AccountData::read_from_bytes(&account_data_file_contents).map_err(|err| err.to_string())?;
    let account_id = account_data.account.id();

    client.import_account(account_data).await?;

    Ok(account_id)
}

// IMPORT NOTE
// ================================================================================================

fn read_note_file(filename: PathBuf) -> Result<NoteFile, String> {
    let mut contents = vec![];
    let mut _file = File::open(filename)
        .and_then(|mut f| f.read_to_end(&mut contents))
        .map_err(|err| err.to_string());

    NoteFile::read_from_bytes(&contents).map_err(|err| err.to_string())
}

// HELPERS
// ================================================================================================

/// Checks that all files exist, otherwise returns an error. It also ensures that all files have a
/// specific extension
fn validate_paths(paths: &[PathBuf]) -> Result<(), String> {
    let invalid_path = paths.iter().find(|path| !path.exists());

    if let Some(path) = invalid_path {
        Err(format!("The path `{}` does not exist", path.to_string_lossy()).to_string())
    } else {
        Ok(())
    }
}

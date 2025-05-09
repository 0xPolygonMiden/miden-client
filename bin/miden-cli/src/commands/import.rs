use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use miden_client::{
    Client, ClientError,
    account::{AccountFile, AccountId},
    note::NoteFile,
    utils::Deserializable,
};
use tracing::info;

use crate::{
    CliKeyStore, Parser, commands::account::maybe_set_default_account, errors::CliError,
    utils::load_config_file,
};

#[derive(Debug, Parser, Clone)]
#[command(about = "Import notes or accounts")]
pub struct ImportCmd {
    /// Paths to the files that contains the account/note data.
    #[arg()]
    filenames: Vec<PathBuf>,
    /// Only relevant for accounts. If set, the account will be overwritten if it already exists.
    #[arg(short, long, default_value_t = false)]
    overwrite: bool,
}

impl ImportCmd {
    pub async fn execute(&self, mut client: Client, keystore: CliKeyStore) -> Result<(), CliError> {
        validate_paths(&self.filenames)?;
        let (mut current_config, _) = load_config_file()?;
        for filename in &self.filenames {
            let note_file = read_note_file(filename.clone());

            if let Ok(note_file) = note_file {
                let note_id = client.import_note(note_file).await?;
                println!("Succesfully imported note {}", note_id.inner());
            } else {
                info!(
                    "Attempting to import account data from {}...",
                    fs::canonicalize(filename)?.as_path().display()
                );
                let account_data_file_contents = fs::read(filename)?;

                let account_id = import_account(
                    &mut client,
                    &keystore,
                    &account_data_file_contents,
                    self.overwrite,
                )
                .await?;

                println!("Successfully imported account {account_id}");

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
    client: &mut Client,
    keystore: &CliKeyStore,
    account_data_file_contents: &[u8],
    overwrite: bool,
) -> Result<AccountId, CliError> {
    let account_data = AccountFile::read_from_bytes(account_data_file_contents)
        .map_err(ClientError::DataDeserializationError)?;
    let account_id = account_data.account.id();

    keystore.add_key(&account_data.auth_secret_key).map_err(CliError::KeyStore)?;

    client
        .add_account(&account_data.account, account_data.account_seed, overwrite)
        .await?;

    Ok(account_id)
}

// IMPORT NOTE
// ================================================================================================

fn read_note_file(filename: PathBuf) -> Result<NoteFile, CliError> {
    let mut contents = vec![];
    let mut _file = File::open(filename).and_then(|mut f| f.read_to_end(&mut contents))?;

    NoteFile::read_from_bytes(&contents)
        .map_err(|err| CliError::Client(ClientError::DataDeserializationError(err)))
}

// HELPERS
// ================================================================================================

/// Checks that all files exist, otherwise returns an error. It also ensures that all files have a
/// specific extension.
fn validate_paths(paths: &[PathBuf]) -> Result<(), CliError> {
    let invalid_path = paths.iter().find(|path| !path.exists());

    if let Some(path) = invalid_path {
        Err(CliError::Input(format!("The path `{}` does not exist", path.to_string_lossy())))
    } else {
        Ok(())
    }
}

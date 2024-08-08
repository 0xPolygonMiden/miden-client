use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use miden_client::{
    accounts::{AccountData, AccountId},
    auth::TransactionAuthenticator,
    crypto::FeltRng,
    notes::{NoteFile, NoteId},
    rpc::NodeRpcClient,
    store::Store,
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
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        mut client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        validate_paths(&self.filenames)?;
        let (mut current_config, _) = load_config_file()?;
        for filename in &self.filenames {
            let note_id = import_note(&mut client, filename.clone()).await;

            if note_id.is_err() {
                println!("Failed to import note from file {}", filename.to_string_lossy());
                println!("With error: {:?}", note_id.clone().err().unwrap());
            };

            if let Ok(note_id) = note_id {
                println!("Succesfully imported note {}", note_id.inner());
            } else {
                let account_id = import_account(&mut client, filename)
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

fn import_account<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: &mut Client<N, R, S, A>,
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

    client.import_account(account_data)?;

    Ok(account_id)
}

// IMPORT NOTE
// ================================================================================================

pub async fn import_note<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: &mut Client<N, R, S, A>,
    filename: PathBuf,
) -> Result<NoteId, String> {
    let mut contents = vec![];
    let mut _file = File::open(filename)
        .and_then(|mut f| f.read_to_end(&mut contents))
        .map_err(|err| err.to_string());

    let note_file = NoteFile::read_from_bytes(&contents).map_err(|err| err.to_string())?;

    client.import_note(note_file).await.map_err(|err| err.to_string())
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

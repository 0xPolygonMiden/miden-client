use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use miden_client::{
    rpc::NodeRpcClient,
    store::{InputNoteRecord, Store},
    Client,
};
use miden_objects::{
    accounts::{AccountData, AccountId},
    crypto::rand::FeltRng,
    notes::{NoteFile, NoteId},
    transaction::InputNote,
    utils::Deserializable,
};
use miden_tx::auth::TransactionAuthenticator;
use tracing::info;

use super::{utils::load_config_file, Parser};
use crate::cli::account::maybe_set_default_account;

#[derive(Debug, Parser, Clone)]
#[clap(about = "Import client objects such as accounts and notes")]
pub struct ImportCmd {
    /// Paths to the files that contains the account/note data
    #[arg()]
    filenames: Vec<PathBuf>,
    /// Skip verification of note's existence in the chain (Only when importing notes)
    #[clap(short, long, default_value = "false")]
    no_verify: bool,
}

impl ImportCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        mut client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        validate_paths(&self.filenames)?;
        let (mut current_config, _) = load_config_file()?;
        for filename in &self.filenames {
            if is_note_file(filename) {
                let note_id = import_note(&mut client, filename.clone(), !self.no_verify)
                    .await
                    .map_err(|_| format!("Failed to parse file {}", filename.to_string_lossy()))?;

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
    verify: bool,
) -> Result<NoteId, String> {
    let mut contents = vec![];
    let mut _file = File::open(filename)
        .and_then(|mut f| f.read_to_end(&mut contents))
        .map_err(|err| err.to_string());

    let note_file = NoteFile::read_from_bytes(&contents).map_err(|err| err.to_string())?;

    let input_note_record: InputNoteRecord = match note_file {
        NoteFile::NoteId(_) => todo!("Importing note ID is not supported yet"),
        NoteFile::NoteDetails(details, _) => (&details).into(),
        NoteFile::NoteWithProof(note, inclusion_proof) => {
            InputNote::authenticated(note, inclusion_proof).into()
        },
    };

    let note_id = input_note_record.id();
    client
        .import_note(input_note_record, verify)
        .await
        .map_err(|err| err.to_string())?;

    Ok(note_id)
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

fn is_note_file(filename: &PathBuf) -> bool {
    let file_contents = fs::read(filename).expect("Filename should exist");
    if file_contents.len() >= 4 {
        let magic_bytes = &file_contents[..4];
        return magic_bytes == b"note";
    }
    false
}

use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use miden_client::{
    client::{rpc::NodeRpcClient, Client},
    store::{InputNoteRecord, Store},
};
use miden_objects::{
    accounts::AccountData, crypto::rand::FeltRng, notes::NoteId, utils::Deserializable,
};
use tracing::info;

use super::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(about = "Import client objects such as accounts and notes")]
pub enum ImportCmd {
    /// Import accounts from binary files (with .mac extension)
    #[clap(short_flag = 'a')]
    Account {
        /// Paths to the files that contains the account data
        #[arg()]
        filenames: Vec<PathBuf>,
    },
    /// Import note data from a binary file
    #[clap(short_flag = 'n')]
    Note {
        /// Path to the file that contains the input note data
        #[clap()]
        filename: PathBuf,

        /// Skip verification of note's existence in the chain
        #[clap(short, long, default_value = "false")]
        no_verify: bool,
    },
}

impl ImportCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store>(
        &self,
        mut client: Client<N, R, S>,
    ) -> Result<(), String> {
        match self {
            ImportCmd::Account { filenames } => {
                validate_paths(filenames, "mac")?;
                for filename in filenames {
                    import_account(&mut client, filename)?;
                }
                println!("Imported {} accounts.", filenames.len());
            },
            ImportCmd::Note { filename, no_verify } => {
                let note_id = import_note(&mut client, filename.clone(), !(*no_verify)).await?;
                println!("Succesfully imported note {}", note_id.inner());
            },
        }
        Ok(())
    }
}

// IMPORT ACCOUNT
// ================================================================================================

fn import_account<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &mut Client<N, R, S>,
    filename: &PathBuf,
) -> Result<(), String> {
    info!(
        "Attempting to import account data from {}...",
        fs::canonicalize(filename).map_err(|err| err.to_string())?.as_path().display()
    );
    let account_data_file_contents = fs::read(filename).map_err(|err| err.to_string())?;
    let account_data =
        AccountData::read_from_bytes(&account_data_file_contents).map_err(|err| err.to_string())?;
    let account_id = account_data.account.id();

    client.import_account(account_data)?;
    println!("Imported account with ID: {}", account_id);

    Ok(())
}

// IMPORT NOTE
// ================================================================================================

pub async fn import_note<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &mut Client<N, R, S>,
    filename: PathBuf,
    verify: bool,
) -> Result<NoteId, String> {
    let mut contents = vec![];
    let mut _file = File::open(filename)
        .and_then(|mut f| f.read_to_end(&mut contents))
        .map_err(|err| err.to_string());

    let input_note_record =
        InputNoteRecord::read_from_bytes(&contents).map_err(|err| err.to_string())?;

    let note_id = input_note_record.id();
    client
        .import_input_note(input_note_record, verify)
        .await
        .map_err(|err| err.to_string())?;

    Ok(note_id)
}

// HELPERS
// ================================================================================================

/// Checks that all files exist, otherwise returns an error. It also ensures that all files have a
/// specific extension
fn validate_paths(paths: &[PathBuf], expected_extension: &str) -> Result<(), String> {
    let invalid_path = paths.iter().find(|path| {
        !path.exists() || path.extension().map_or(false, |ext| ext != expected_extension)
    });

    if let Some(path) = invalid_path {
        Err(format!(
            "The path `{}` does not exist or does not have the appropiate extension",
            path.to_string_lossy()
        )
        .to_string())
    } else {
        Ok(())
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use miden_client::{
        client::get_random_coin,
        config::{ClientConfig, Endpoint, RpcConfig},
        errors::IdPrefixFetchError,
        mock::{mock_full_chain_mmr_and_notes, mock_notes, MockClient, MockRpcApi},
        store::{sqlite_store::SqliteStore, InputNoteRecord},
    };
    use miden_lib::transaction::TransactionKernel;
    use uuid::Uuid;

    use super::import_note;
    use crate::cli::{get_note_with_id_prefix, input_notes::export_note};

    #[tokio::test]
    async fn import_export_recorded_note() {
        // generate test client
        let mut path = temp_dir();
        path.push(Uuid::new_v4().to_string());
        let client_config = ClientConfig::new(
            path.into_os_string().into_string().unwrap().try_into().unwrap(),
            RpcConfig::default(),
        );

        let rng = get_random_coin();
        let store = SqliteStore::new((&client_config).into()).unwrap();

        let mut client =
            MockClient::new(MockRpcApi::new(&Endpoint::default().to_string()), rng, store, true);

        // generate test data
        let assembler = TransactionKernel::assembler();
        let (consumed_notes, created_notes) = mock_notes(&assembler);
        let (_, committed_notes, ..) = mock_full_chain_mmr_and_notes(consumed_notes);

        let committed_note: InputNoteRecord = committed_notes.first().unwrap().clone().into();
        let pending_note = InputNoteRecord::from(created_notes.first().unwrap().clone());

        client.import_input_note(committed_note.clone(), false).await.unwrap();
        assert!(client.import_input_note(pending_note.clone(), true).await.is_err());
        client.import_input_note(pending_note.clone(), false).await.unwrap();
        assert!(pending_note.inclusion_proof().is_none());
        assert!(committed_note.inclusion_proof().is_some());

        let mut filename_path = temp_dir();
        filename_path.push("test_import");

        let mut filename_path_pending = temp_dir();
        filename_path_pending.push("test_import_pending");

        export_note(&client, &committed_note.id().inner().to_string(), Some(filename_path.clone()))
            .unwrap();

        assert!(filename_path.exists());

        export_note(
            &client,
            &pending_note.id().inner().to_string(),
            Some(filename_path_pending.clone()),
        )
        .unwrap();

        assert!(filename_path_pending.exists());

        // generate test client to import notes to
        let mut path = temp_dir();
        path.push(Uuid::new_v4().to_string());
        let client_config = ClientConfig::new(
            path.into_os_string().into_string().unwrap().try_into().unwrap(),
            RpcConfig::default(),
        );
        let store = SqliteStore::new((&client_config).into()).unwrap();

        let mut client =
            MockClient::new(MockRpcApi::new(&Endpoint::default().to_string()), rng, store, true);

        import_note(&mut client, filename_path, false).await.unwrap();
        let imported_note_record: InputNoteRecord =
            client.get_input_note(committed_note.id()).unwrap();

        assert_eq!(committed_note.id(), imported_note_record.id());

        import_note(&mut client, filename_path_pending, false).await.unwrap();
        let imported_pending_note_record = client.get_input_note(pending_note.id()).unwrap();

        assert_eq!(imported_pending_note_record.id(), pending_note.id());
    }

    #[tokio::test]
    async fn get_input_note_with_prefix() {
        // generate test client
        let mut path = temp_dir();
        path.push(Uuid::new_v4().to_string());
        let client_config = ClientConfig::new(
            path.into_os_string().into_string().unwrap().try_into().unwrap(),
            RpcConfig::default(),
        );

        let rng = get_random_coin();
        let store = SqliteStore::new((&client_config).into()).unwrap();

        let mut client =
            MockClient::new(MockRpcApi::new(&Endpoint::default().to_string()), rng, store, true);

        // Ensure we get an error if no note is found
        let non_existent_note_id = "0x123456";
        assert_eq!(
            get_note_with_id_prefix(&client, non_existent_note_id),
            Err(IdPrefixFetchError::NoMatch(
                format!("note ID prefix {non_existent_note_id}").to_string()
            ))
        );

        // generate test data
        let assembler = TransactionKernel::assembler();
        let (consumed_notes, created_notes) = mock_notes(&assembler);
        let (_, notes, ..) = mock_full_chain_mmr_and_notes(consumed_notes);

        let committed_note: InputNoteRecord = notes.first().unwrap().clone().into();
        let pending_note = InputNoteRecord::from(created_notes.first().unwrap().clone());

        client.import_input_note(committed_note.clone(), false).await.unwrap();
        client.import_input_note(pending_note.clone(), false).await.unwrap();
        assert!(pending_note.inclusion_proof().is_none());
        assert!(committed_note.inclusion_proof().is_some());

        // Check that we can fetch Both notes
        let note = get_note_with_id_prefix(&client, &committed_note.id().to_hex()).unwrap();
        assert_eq!(note.id(), committed_note.id());

        let note = get_note_with_id_prefix(&client, &pending_note.id().to_hex()).unwrap();
        assert_eq!(note.id(), pending_note.id());

        // Check that we get an error if many match
        let note_id_with_many_matches = "0x";
        assert_eq!(
            get_note_with_id_prefix(&client, note_id_with_many_matches),
            Err(IdPrefixFetchError::MultipleMatches(
                format!("note ID prefix {note_id_with_many_matches}").to_string()
            ))
        );
    }
}

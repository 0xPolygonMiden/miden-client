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
    notes::NoteId,
    utils::Deserializable,
};
use miden_tx::auth::TransactionAuthenticator;
use tracing::info;

use super::Parser;

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
        for filename in &self.filenames {
            let note_id = import_note(&mut client, filename.clone(), !self.no_verify).await;
            if note_id.is_ok() {
                println!("Succesfully imported note {}", note_id.unwrap().inner());
                continue;
            }
            let account_id = import_account(&mut client, filename)
                .map_err(|_| format!("Failed to parse file {}", filename.to_string_lossy()))?;
            println!("Succesfully imported account {}", account_id);
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
fn validate_paths(paths: &[PathBuf]) -> Result<(), String> {
    let invalid_path = paths.iter().find(|path| !path.exists());

    if let Some(path) = invalid_path {
        Err(format!("The path `{}` does not exist", path.to_string_lossy()).to_string())
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
        errors::IdPrefixFetchError,
        mock::{
            create_test_client, mock_full_chain_mmr_and_notes, mock_fungible_faucet_account,
            mock_notes,
        },
        store::{InputNoteRecord, NoteFilter},
        transactions::transaction_request::TransactionTemplate,
    };
    use miden_lib::transaction::TransactionKernel;
    use miden_objects::{
        accounts::{
            account_id::testing::ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, AccountId, AuthSecretKey,
        },
        assets::FungibleAsset,
        crypto::dsa::rpo_falcon512::SecretKey,
        notes::Note,
    };

    use super::import_note;
    use crate::cli::{export::export_note, get_input_note_with_id_prefix};

    #[tokio::test]
    async fn import_export_recorded_note() {
        // This test will run a mint transaction that creates an output note and we'll try
        // exporting that note and then importing it. So the client's state should be:
        //
        // 1. No notes at all
        // 2. One output note
        // 3. One output note, one input note. Both representing the same note.

        // generate test client
        let mut client = create_test_client();

        // Add a faucet account to run a mint tx against it
        const FAUCET_ID: u64 = ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN;
        const INITIAL_BALANCE: u64 = 1000;
        let key_pair = SecretKey::new();

        let faucet = mock_fungible_faucet_account(
            AccountId::try_from(FAUCET_ID).unwrap(),
            INITIAL_BALANCE,
            key_pair.clone(),
        );

        client.sync_state().await.unwrap();
        client
            .insert_account(&faucet, None, &AuthSecretKey::RpoFalcon512(key_pair))
            .unwrap();

        // Ensure client has no notes
        assert!(client.get_input_notes(NoteFilter::All).unwrap().is_empty());
        assert!(client.get_output_notes(NoteFilter::All).unwrap().is_empty());

        // mint asset to create an output note
        // using a random account id will mean that the note won't be included in the input notes
        // table.
        let transaction_template = TransactionTemplate::MintFungibleAsset(
            FungibleAsset::new(faucet.id(), 5u64).unwrap(),
            AccountId::from_hex("0x168187d729b31a84").unwrap(),
            miden_objects::notes::NoteType::OffChain,
        );

        let transaction_request = client.build_transaction_request(transaction_template).unwrap();
        let transaction = client.new_transaction(transaction_request).unwrap();
        let created_note = transaction.created_notes().get_note(0).clone();
        let proven_transaction =
            client.prove_transaction(transaction.executed_transaction().clone()).unwrap();

        client.submit_transaction(transaction, proven_transaction).await.unwrap();

        // Ensure client has no input notes and one output note
        assert!(client.get_input_notes(NoteFilter::All).unwrap().is_empty());
        assert!(!client.get_output_notes(NoteFilter::All).unwrap().is_empty());
        let exported_note = client
            .get_output_notes(NoteFilter::Unique(created_note.id()))
            .unwrap()
            .pop()
            .unwrap();

        // export the note with the CLI function
        let mut filename_path = temp_dir();
        filename_path.push("test_import");
        export_note(&client, &exported_note.id().to_hex(), Some(filename_path.clone())).unwrap();

        // Try importing the same note with the CLI function
        let imported_note_id = import_note(&mut client, filename_path, false).await.unwrap();

        // Ensure client has one input note and one output note
        assert_eq!(client.get_input_notes(NoteFilter::All).unwrap().len(), 1);
        assert_eq!(client.get_output_notes(NoteFilter::All).unwrap().len(), 1);

        let imported_note = client
            .get_input_notes(NoteFilter::Unique(imported_note_id))
            .unwrap()
            .pop()
            .unwrap();

        let exported_note: InputNoteRecord = exported_note.try_into().unwrap();
        let exported_note: Note = exported_note.try_into().unwrap();
        let imported_note: Note = imported_note.try_into().unwrap();

        assert_eq!(exported_note, imported_note);
    }

    #[tokio::test]
    async fn get_input_note_with_prefix() {
        // generate test client
        let mut client = create_test_client();

        // Ensure we get an error if no note is found
        let non_existent_note_id = "0x123456";
        assert_eq!(
            get_input_note_with_id_prefix(&client, non_existent_note_id),
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
        let note = get_input_note_with_id_prefix(&client, &committed_note.id().to_hex()).unwrap();
        assert_eq!(note.id(), committed_note.id());

        let note = get_input_note_with_id_prefix(&client, &pending_note.id().to_hex()).unwrap();
        assert_eq!(note.id(), pending_note.id());

        // Check that we get an error if many match
        let note_id_with_many_matches = "0x";
        assert_eq!(
            get_input_note_with_id_prefix(&client, note_id_with_many_matches),
            Err(IdPrefixFetchError::MultipleMatches(
                format!("note ID prefix {note_id_with_many_matches}").to_string()
            ))
        );
    }
}

use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use clap::ValueEnum;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
use miden_client::{
    client::{rpc::NodeRpcClient, ConsumableNote},
    errors::ClientError,
    store::{InputNoteRecord, NoteFilter as ClientNoteFilter, Store},
};
use miden_objects::{
    accounts::AccountId,
    crypto::rand::FeltRng,
    notes::{NoteId, NoteInputs},
    Digest,
};
use miden_tx::utils::{Deserializable, Serializable};

use super::{Client, Parser};
use crate::cli::{create_dynamic_table, get_note_with_id_prefix};

#[derive(Clone, Debug, ValueEnum)]
pub enum NoteFilter {
    Pending,
    Committed,
    Consumed,
}

#[derive(Debug, Parser, Clone)]
#[clap(about = "View and manage input notes")]
pub enum InputNotes {
    /// List input notes
    #[clap(short_flag = 'l')]
    List {
        /// Filter the displayed note list
        #[clap(short, long)]
        filter: Option<NoteFilter>,
    },

    /// Show details of the input note for the specified note ID
    #[clap(short_flag = 's')]
    Show {
        /// Note ID of the input note to show
        #[clap()]
        id: String,

        /// Show note script
        #[clap(short, long, default_value = "false")]
        script: bool,

        /// Show note vault
        #[clap(short, long, default_value = "false")]
        vault: bool,

        /// Show note inputs
        #[clap(short, long, default_value = "false")]
        inputs: bool,
    },

    /// Export input note data to a binary file
    #[clap(short_flag = 'e')]
    Export {
        /// Note ID of the input note to show
        #[clap()]
        id: String,

        /// Path to the file that will contain the input note data. If not provided, the filename will be the input note ID
        #[clap()]
        filename: Option<PathBuf>,
    },

    /// Import input note data from a binary file
    #[clap(short_flag = 'i')]
    Import {
        /// Path to the file that contains the input note data
        #[clap()]
        filename: PathBuf,

        /// Skip verification of note's existence in the chain
        #[clap(short, long, default_value = "false")]
        no_verify: bool,
    },

    /// List consumable input notes
    #[clap(short_flag = 'c')]
    ListConsumable {
        /// Account ID used to filter list. Only notes consumable by this account will be shown.
        #[clap()]
        account_id: Option<String>,
    },
}

impl InputNotes {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store>(
        &self,
        mut client: Client<N, R, S>,
    ) -> Result<(), String> {
        match self {
            InputNotes::List { filter } => {
                let filter = match filter {
                    Some(NoteFilter::Committed) => ClientNoteFilter::Committed,
                    Some(NoteFilter::Consumed) => ClientNoteFilter::Consumed,
                    Some(NoteFilter::Pending) => ClientNoteFilter::Pending,
                    None => ClientNoteFilter::All,
                };

                list_input_notes(client, filter)?;
            },
            InputNotes::Show { id, script, vault, inputs } => {
                show_input_note(client, id.to_owned(), *script, *vault, *inputs)?;
            },
            InputNotes::Export { id, filename } => {
                export_note(&client, id, filename.clone())?;
                println!("Succesfully exported note {}", id);
            },
            InputNotes::Import { filename, no_verify } => {
                let note_id = import_note(&mut client, filename.clone(), !(*no_verify)).await?;
                println!("Succesfully imported note.");
                println!(
                    "To view note details execute `miden-client input-notes -s {}`",
                    note_id.inner()
                );
            },
            InputNotes::ListConsumable { account_id } => {
                list_consumable_notes(client, account_id)?;
            },
        }
        Ok(())
    }
}

// LIST INPUT NOTES
// ================================================================================================
fn list_input_notes<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: Client<N, R, S>,
    filter: ClientNoteFilter,
) -> Result<(), String> {
    let notes = client.get_input_notes(filter)?;
    print_notes_summary(&notes)?;
    Ok(())
}

// EXPORT INPUT NOTE
// ================================================================================================
pub fn export_note<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &Client<N, R, S>,
    note_id: &str,
    filename: Option<PathBuf>,
) -> Result<File, String> {
    let note_id = Digest::try_from(note_id)
        .map_err(|err| format!("Failed to parse input note id: {}", err))?
        .into();
    let output_note = client
        .get_output_notes(miden_client::store::NoteFilter::Unique(note_id))?
        .pop()
        .expect("should have an output note");

    // Convert output note into input note before exporting
    let input_note: InputNoteRecord = output_note
        .try_into()
        .map_err(|_err| format!("Can't export note with ID {}", note_id.to_hex()))?;

    let file_path = filename.unwrap_or_else(|| {
        let mut dir = PathBuf::new();
        dir.push(note_id.inner().to_string());
        dir
    });

    let mut file = File::create(file_path).map_err(|err| err.to_string())?;

    file.write_all(&input_note.to_bytes()).map_err(|err| err.to_string())?;

    Ok(file)
}

// IMPORT INPUT NOTE
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

// SHOW INPUT NOTE
// ================================================================================================
fn show_input_note<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: Client<N, R, S>,
    note_id: String,
    show_script: bool,
    show_vault: bool,
    show_inputs: bool,
) -> Result<(), String> {
    let input_note_record =
        get_note_with_id_prefix(&client, &note_id).map_err(|err| err.to_string())?;

    // print note summary
    print_notes_summary(core::iter::once(&input_note_record))?;

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_HORIZONTAL_ONLY)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth);

    // print note script
    if show_script {
        let script = input_note_record.details().script();

        table
            .add_row(vec![
                Cell::new("Note Script hash").add_attribute(Attribute::Bold),
                Cell::new(script.hash()),
            ])
            .add_row(vec![
                Cell::new("Note Script code").add_attribute(Attribute::Bold),
                Cell::new(script.code()),
            ]);
    };

    // print note vault
    if show_vault {
        table
            .add_row(vec![
                Cell::new("Note Vault hash").add_attribute(Attribute::Bold),
                Cell::new(input_note_record.assets().commitment()),
            ])
            .add_row(vec![Cell::new("Note Vault").add_attribute(Attribute::Bold)]);

        input_note_record.assets().iter().for_each(|asset| {
            table.add_row(vec![Cell::new(format!("{:?}", asset))]);
        })
    };

    if show_inputs {
        let inputs = NoteInputs::new(input_note_record.details().inputs().clone())
            .map_err(ClientError::NoteError)?;

        table
            .add_row(vec![
                Cell::new("Note Inputs hash").add_attribute(Attribute::Bold),
                Cell::new(inputs.commitment()),
            ])
            .add_row(vec![Cell::new("Note Inputs").add_attribute(Attribute::Bold)]);

        inputs.values().iter().enumerate().for_each(|(idx, input)| {
            table.add_row(vec![Cell::new(idx).add_attribute(Attribute::Bold), Cell::new(input)]);
        });
    };

    println!("{table}");
    Ok(())
}

// LIST CONSUMABLE INPUT NOTES
// ================================================================================================
fn list_consumable_notes<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: Client<N, R, S>,
    account_id: &Option<String>,
) -> Result<(), String> {
    let account_id = match account_id {
        Some(id) => Some(AccountId::from_hex(id.as_str()).map_err(|err| err.to_string())?),
        None => None,
    };
    let notes = client.get_consumable_notes(account_id)?;
    print_consumable_notes_summary(&notes)?;
    Ok(())
}

// HELPERS
// ================================================================================================
fn print_notes_summary<'a, I>(notes: I) -> Result<(), String>
where
    I: IntoIterator<Item = &'a InputNoteRecord>,
{
    let mut table = create_dynamic_table(&[
        "Note ID",
        "Script Hash",
        "Vault Vash",
        "Inputs Hash",
        "Serial Num",
        "Type",
        "Commit Height",
    ]);

    for input_note_record in notes {
        let commit_height = input_note_record
            .inclusion_proof()
            .map(|proof| proof.origin().block_num.to_string())
            .unwrap_or("-".to_string());

        let script = input_note_record.details().script();

        let inputs = NoteInputs::new(input_note_record.details().inputs().clone())
            .map_err(ClientError::NoteError)?;

        table.add_row(vec![
            input_note_record.id().inner().to_string(),
            script.hash().to_string(),
            input_note_record.assets().commitment().to_string(),
            inputs.commitment().to_string(),
            Digest::new(input_note_record.details().serial_num()).to_string(),
            note_record_type(input_note_record),
            commit_height,
        ]);
    }

    println!("{table}");

    Ok(())
}

fn print_consumable_notes_summary<'a, I>(notes: I) -> Result<(), String>
where
    I: IntoIterator<Item = &'a ConsumableNote>,
{
    let mut table = create_dynamic_table(&["Note ID", "Account ID", "Relevance"]);

    for consumable_note in notes {
        for relevance in &consumable_note.relevances {
            table.add_row(vec![
                consumable_note.note.id().to_hex(),
                relevance.0.to_string(),
                relevance.1.to_string(),
            ]);
        }
    }

    println!("{table}");

    Ok(())
}

fn note_record_type(note_record: &InputNoteRecord) -> String {
    match note_record.metadata() {
        Some(metadata) => match metadata.note_type() {
            miden_objects::notes::NoteType::OffChain => "OffChain",
            miden_objects::notes::NoteType::Encrypted => "Encrypted",
            miden_objects::notes::NoteType::Public => "Public",
        },
        None => "-",
    }
    .to_string()
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use miden_client::{
        client::{get_random_coin, transactions::transaction_request::TransactionTemplate},
        config::{ClientConfig, Endpoint, RpcConfig},
        errors::IdPrefixFetchError,
        mock::{
            mock_full_chain_mmr_and_notes, mock_fungible_faucet_account, mock_notes, MockClient,
            MockRpcApi,
        },
        store::{sqlite_store::SqliteStore, AuthInfo, InputNoteRecord, NoteFilter},
    };
    use miden_lib::transaction::TransactionKernel;
    use miden_objects::{
        accounts::{AccountId, ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN},
        assets::FungibleAsset,
        crypto::dsa::rpo_falcon512::SecretKey,
        notes::Note,
    };
    use uuid::Uuid;

    use crate::cli::{
        get_note_with_id_prefix,
        input_notes::{export_note, import_note},
    };

    #[tokio::test]
    async fn test_import_note_validation() {
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
    }

    #[tokio::test]
    async fn import_export_recorded_note() {
        // This test will run a mint transaction that creates an output note and we'll try
        // exporting that note and then importing it. So the client's state should be:
        //
        // 1. No notes at all
        // 2. One output note
        // 3. One output note, one input note. Both representing the same note.

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
        client.insert_account(&faucet, None, &AuthInfo::RpoFalcon512(key_pair)).unwrap();

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
        let created_note = transaction.created_notes()[0].clone();
        client.submit_transaction(transaction).await.unwrap();

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
        println!("exporting note to {}", filename_path.to_string_lossy());
        export_note(&client, &exported_note.id().to_hex(), Some(filename_path.clone())).unwrap();
        println!("exported!");

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

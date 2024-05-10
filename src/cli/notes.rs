use std::collections::{HashMap, HashSet};

use clap::ValueEnum;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
use miden_client::{
    client::{rpc::NodeRpcClient, ConsumableNote},
    errors::{ClientError, IdPrefixFetchError},
    store::{InputNoteRecord, NoteFilter as ClientNoteFilter, OutputNoteRecord, Store},
};
use miden_objects::{
    accounts::AccountId,
    crypto::rand::FeltRng,
    notes::{NoteInputs, NoteMetadata},
    Digest,
};
use miden_tx::TransactionAuthenticator;

use super::{Client, Parser};
use crate::cli::{
    create_dynamic_table, get_input_note_with_id_prefix, get_output_note_with_id_prefix,
};

#[derive(Clone, Debug, ValueEnum)]
pub enum NoteFilter {
    Pending,
    Committed,
    Consumed,
}

#[derive(Debug, Parser, Clone)]
#[clap(about = "View and manage notes")]
pub enum Notes {
    /// List notes
    #[clap(short_flag = 'l')]
    List {
        /// Filter the displayed note list
        #[clap(short, long)]
        filter: Option<NoteFilter>,
    },

    /// Show details of the note for the specified note ID
    #[clap(short_flag = 's')]
    Show {
        /// Note ID of the note to show
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

    /// List consumable notes
    #[clap(short_flag = 'c')]
    ListConsumable {
        /// Account ID used to filter list. Only notes consumable by this account will be shown.
        #[clap()]
        account_id: Option<String>,
    },
}

impl Default for Notes {
    fn default() -> Self {
        Notes::List { filter: None }
    }
}

impl Notes {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        match self {
            Notes::List { filter } => {
                let filter = match filter {
                    Some(NoteFilter::Committed) => ClientNoteFilter::Committed,
                    Some(NoteFilter::Consumed) => ClientNoteFilter::Consumed,
                    Some(NoteFilter::Pending) => ClientNoteFilter::Pending,
                    None => ClientNoteFilter::All,
                };

                list_notes(client, filter)?;
            },
            Notes::Show { id, script, vault, inputs } => {
                show_note(client, id.to_owned(), *script, *vault, *inputs)?;
            },
            Notes::ListConsumable { account_id } => {
                list_consumable_notes(client, account_id)?;
            },
        }
        Ok(())
    }
}

// LIST NOTES
// ================================================================================================
fn list_notes<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: Client<N, R, S, A>,
    filter: ClientNoteFilter,
) -> Result<(), String> {
    let input_notes = client.get_input_notes(filter.clone())?;
    let output_notes = client.get_output_notes(filter.clone())?;

    let mut all_note_ids = HashSet::new();
    let mut input_note_records = HashMap::new();
    let mut output_note_records = HashMap::new();

    for note in input_notes {
        all_note_ids.insert(note.id().to_hex());
        input_note_records.insert(note.id().to_hex(), note);
    }

    for note in output_notes {
        all_note_ids.insert(note.id().to_hex());
        output_note_records.insert(note.id().to_hex(), note);
    }

    let zipped_notes = all_note_ids
        .iter()
        .map(|note_id| (input_note_records.get(note_id), output_note_records.get(note_id)));

    print_notes_summary(zipped_notes)?;
    Ok(())
}

// SHOW NOTE
// ================================================================================================
fn show_note<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: Client<N, R, S, A>,
    note_id: String,
    show_script: bool,
    show_vault: bool,
    show_inputs: bool,
) -> Result<(), String> {
    let input_note_record = get_input_note_with_id_prefix(&client, &note_id);
    let output_note_record = get_output_note_with_id_prefix(&client, &note_id);

    // If we don't find an input note nor an output note return an error
    if matches!(input_note_record, Err(IdPrefixFetchError::NoMatch(_)))
        && matches!(output_note_record, Err(IdPrefixFetchError::NoMatch(_)))
    {
        return Err("Couldn't find notes matching the specified note ID".to_string());
    }

    // If either one of the two match with multiple notes return an error
    if matches!(input_note_record, Err(IdPrefixFetchError::MultipleMatches(_)))
        || matches!(output_note_record, Err(IdPrefixFetchError::MultipleMatches(_)))
    {
        return Err("The specified note ID hex prefix matched with more than one note.".to_string());
    }

    let input_note_record = input_note_record.ok();
    let output_note_record = output_note_record.ok();

    // If we match one note as the input note and another one as the output note return an error
    match (&input_note_record, &output_note_record) {
        (Some(input_record), Some(output_record)) if input_record.id() != output_record.id() => {
            return Err(
                "The specified note ID hex prefix matched with more than one note.".to_string()
            );
        },
        _ => {},
    }

    // print note summary
    print_notes_summary(core::iter::once((
        input_note_record.as_ref(),
        output_note_record.as_ref(),
    )))?;

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_HORIZONTAL_ONLY)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth);

    let (script, inputs) = match (&input_note_record, &output_note_record) {
        (Some(record), _) => {
            let details = record.details();
            (Some(details.script().clone()), Some(details.inputs().clone()))
        },
        (_, Some(record)) => {
            let details = record.details();
            (
                details.map(|details| details.script().clone()),
                details.map(|details| details.inputs().clone()),
            )
        },
        (None, None) => {
            panic!("One of the two records should be Some")
        },
    };

    let assets = input_note_record
        .map(|record| record.assets().clone())
        .or(output_note_record.map(|record| record.assets().clone()))
        .expect("One of the two records should be Some");

    // print note script
    if show_script && script.is_some() {
        let script = script.expect("Script should be Some");

        table.add_row(vec![
            Cell::new("Note Script code").add_attribute(Attribute::Bold),
            Cell::new(script.code()),
        ]);
    };

    // print note vault
    if show_vault {
        table.add_row(vec![Cell::new("Note Vault").add_attribute(Attribute::Bold)]);

        assets.iter().for_each(|asset| {
            table.add_row(vec![Cell::new(format!("{:?}", asset))]);
        })
    };

    if show_inputs && inputs.is_some() {
        let inputs = inputs.expect("Inputs should be Some");
        let inputs = NoteInputs::new(inputs.clone()).map_err(ClientError::NoteError)?;

        table.add_row(vec![Cell::new("Note Inputs").add_attribute(Attribute::Bold)]);

        inputs.values().iter().enumerate().for_each(|(idx, input)| {
            table.add_row(vec![Cell::new(idx).add_attribute(Attribute::Bold), Cell::new(input)]);
        });
    };

    println!("{table}");
    Ok(())
}

// LIST CONSUMABLE INPUT NOTES
// ================================================================================================
fn list_consumable_notes<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: Client<N, R, S, A>,
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
    I: IntoIterator<Item = (Option<&'a InputNoteRecord>, Option<&'a OutputNoteRecord>)>,
{
    let mut table = create_dynamic_table(&[
        "Note ID",
        "Script Hash",
        "Vault Hash",
        "Inputs Hash",
        "Serial Num",
        "Type",
        "Commit Height",
        "Exportable?",
    ]);

    for (input_note_record, output_note_record) in notes {
        let note_id = input_note_record
            .map(|record| record.id())
            .or(output_note_record.map(|record| record.id()))
            .expect("One of the two records should be Some");

        let commit_height = input_note_record
            .map(|record| {
                record
                    .inclusion_proof()
                    .map(|proof| proof.origin().block_num.to_string())
                    .unwrap_or("-".to_string())
            })
            .or(output_note_record.map(|record| {
                record
                    .inclusion_proof()
                    .map(|proof| proof.origin().block_num.to_string())
                    .unwrap_or("-".to_string())
            }))
            .expect("One of the two records should be Some");

        let assets_str = input_note_record
            .map(|record| record.assets().commitment().to_string())
            .or(output_note_record.map(|record| record.assets().commitment().to_string()))
            .expect("One of the two records should be Some");

        let (inputs_commitment_str, serial_num, script) =
            match (input_note_record, output_note_record) {
                (Some(record), _) => {
                    let details = record.details();
                    (
                        NoteInputs::new(details.inputs().clone())
                            .map_err(ClientError::NoteError)?
                            .commitment()
                            .to_string(),
                        Digest::new(details.serial_num()).to_string(),
                        details.script().hash().to_string(),
                    )
                },
                (None, Some(record)) if record.details().is_some() => {
                    let details = record.details().expect("output record should have details");
                    (
                        NoteInputs::new(details.inputs().clone())
                            .map_err(ClientError::NoteError)?
                            .commitment()
                            .to_string(),
                        Digest::new(details.serial_num()).to_string(),
                        details.script().hash().to_string(),
                    )
                },
                (None, Some(_record)) => ("-".to_string(), "-".to_string(), "-".to_string()),
                (None, None) => panic!("One of the two records should be Some"),
            };

        let note_type = note_record_type(
            input_note_record
                .and_then(|record| record.metadata())
                .or(output_note_record.map(|record| record.metadata())),
        );

        let exportable = if output_note_record.is_some() { "✔" } else { "✘" };

        table.add_row(vec![
            note_id.inner().to_string(),
            script,
            assets_str,
            inputs_commitment_str,
            serial_num,
            note_type,
            commit_height,
            exportable.to_string(),
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

fn note_record_type(note_record_metadata: Option<&NoteMetadata>) -> String {
    match note_record_metadata {
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
    use std::{env::temp_dir, rc::Rc};

    use miden_client::{
        client::{
            authenticator::StoreAuthenticator, get_random_coin,
            transactions::transaction_request::TransactionTemplate,
        },
        config::{ClientConfig, Endpoint, RpcConfig},
        errors::IdPrefixFetchError,
        mock::{
            mock_full_chain_mmr_and_notes, mock_fungible_faucet_account, mock_notes, MockClient,
            MockRpcApi,
        },
        store::{sqlite_store::SqliteStore, InputNoteRecord, NoteFilter},
    };
    use miden_lib::transaction::TransactionKernel;
    use miden_objects::{
        accounts::{account_id::testing::ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, AccountId},
        assets::FungibleAsset,
        crypto::dsa::rpo_falcon512::SecretKey,
        notes::Note,
    };
    use miden_tx::AuthSecretKey;
    use uuid::Uuid;

    use crate::cli::{export::export_note, get_input_note_with_id_prefix, import::import_note};

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

        let store = {
            let sqlite_store = SqliteStore::new((&client_config).into()).unwrap();
            Rc::new(sqlite_store)
        };

        let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);
        let mut client = MockClient::new(
            MockRpcApi::new(&Endpoint::default().to_string()),
            rng,
            store,
            authenticator,
            true,
        );

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
        let store = {
            let sqlite_store = SqliteStore::new((&client_config).into()).unwrap();
            Rc::new(sqlite_store)
        };
        let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);
        let mut client = MockClient::new(
            MockRpcApi::new(&Endpoint::default().to_string()),
            rng,
            store,
            authenticator,
            true,
        );

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
        let store = {
            let sqlite_store = SqliteStore::new((&client_config).into()).unwrap();
            Rc::new(sqlite_store)
        };
        let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);
        let mut client = MockClient::new(
            MockRpcApi::new(&Endpoint::default().to_string()),
            rng,
            store,
            authenticator,
            true,
        );

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

use super::{Client, Parser};
use crate::cli::{create_dynamic_table, get_note_with_id_prefix};
use clap::ValueEnum;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
use crypto::utils::{Deserializable, Serializable};
use miden_client::{
    client::rpc::NodeRpcClient,
    store::{InputNoteRecord, NoteFilter as ClientNoteFilter, Store},
};
use objects::{notes::NoteId, Digest};
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

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
    },
}

impl InputNotes {
    pub fn execute<N: NodeRpcClient, S: Store>(
        &self,
        mut client: Client<N, S>,
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
            }
            InputNotes::Show {
                id,
                script,
                vault,
                inputs,
            } => {
                show_input_note(client, id.to_owned(), *script, *vault, *inputs)?;
            }
            InputNotes::Export { id, filename } => {
                export_note(&client, id, filename.clone())?;
                println!("Succesfully exported note {}", id);
            }
            InputNotes::Import { filename } => {
                let note_id = import_note(&mut client, filename.clone())?;
                println!("Succesfully imported note {}", note_id.inner());
            }
        }
        Ok(())
    }
}

// LIST INPUT NOTES
// ================================================================================================
fn list_input_notes<N: NodeRpcClient, S: Store>(
    client: Client<N, S>,
    filter: ClientNoteFilter,
) -> Result<(), String> {
    let notes = client.get_input_notes(filter)?;
    print_notes_summary(&notes);
    Ok(())
}

// EXPORT INPUT NOTE
// ================================================================================================
pub fn export_note<N: NodeRpcClient, S: Store>(
    client: &Client<N, S>,
    note_id: &str,
    filename: Option<PathBuf>,
) -> Result<File, String> {
    let note_id = Digest::try_from(note_id)
        .map_err(|err| format!("Failed to parse input note id: {}", err))?
        .into();
    let note = client.get_input_note(note_id)?;

    let file_path = filename.unwrap_or_else(|| {
        let mut dir = PathBuf::new();
        dir.push(note_id.inner().to_string());
        dir
    });

    let mut file = File::create(file_path).map_err(|err| err.to_string())?;

    file.write_all(&note.to_bytes())
        .map_err(|err| err.to_string())?;

    Ok(file)
}

// IMPORT INPUT NOTE
// ================================================================================================
pub fn import_note<N: NodeRpcClient, S: Store>(
    client: &mut Client<N, S>,
    filename: PathBuf,
) -> Result<NoteId, String> {
    let mut contents = vec![];
    let mut _file = File::open(filename)
        .and_then(|mut f| f.read_to_end(&mut contents))
        .map_err(|err| err.to_string());

    // TODO: When importing a RecordedNote we want to make sure that the note actually exists in the chain (RPC call)
    // and start monitoring its nullifiers (ie, update the list of relevant tags in the state sync table)
    let input_note_record =
        InputNoteRecord::read_from_bytes(&contents).map_err(|err| err.to_string())?;

    let note_id = input_note_record.note().id();
    client.import_input_note(input_note_record)?;

    Ok(note_id)
}

// SHOW INPUT NOTE
// ================================================================================================
fn show_input_note<N: NodeRpcClient, S: Store>(
    client: Client<N, S>,
    note_id: String,
    show_script: bool,
    show_vault: bool,
    show_inputs: bool,
) -> Result<(), String> {
    let input_note_record =
        get_note_with_id_prefix(&client, &note_id).map_err(|err| err.to_string())?;

    // print note summary
    print_notes_summary(core::iter::once(&input_note_record));

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_HORIZONTAL_ONLY)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth);

    // print note script
    if show_script {
        table
            .add_row(vec![
                Cell::new("Note Script hash").add_attribute(Attribute::Bold),
                Cell::new(input_note_record.note().script().hash()),
            ])
            .add_row(vec![
                Cell::new("Note Script code").add_attribute(Attribute::Bold),
                Cell::new(input_note_record.note().script().code()),
            ]);
    };

    // print note vault
    if show_vault {
        table
            .add_row(vec![
                Cell::new("Note Vault hash").add_attribute(Attribute::Bold),
                Cell::new(input_note_record.note().assets().commitment()),
            ])
            .add_row(vec![Cell::new("Note Vault").add_attribute(Attribute::Bold)]);

        input_note_record.note().assets().iter().for_each(|asset| {
            table.add_row(vec![Cell::new(format!("{:?}", asset))]);
        })
    };

    if show_inputs {
        table
            .add_row(vec![
                Cell::new("Note Inputs hash").add_attribute(Attribute::Bold),
                Cell::new(input_note_record.note().inputs().commitment()),
            ])
            .add_row(vec![Cell::new("Note Inputs").add_attribute(Attribute::Bold)]);
        input_note_record
            .note()
            .inputs()
            .values()
            .iter()
            .enumerate()
            .for_each(|(idx, input)| {
                table.add_row(vec![
                    Cell::new(idx).add_attribute(Attribute::Bold),
                    Cell::new(input),
                ]);
            });
    };

    println!("{table}");
    Ok(())
}

// HELPERS
// ================================================================================================
fn print_notes_summary<'a, I>(notes: I)
where
    I: IntoIterator<Item = &'a InputNoteRecord>,
{
    let mut table = create_dynamic_table(&[
        "Note ID",
        "Script Hash",
        "Vault Vash",
        "Inputs Hash",
        "Serial Num",
        "Commit Height",
    ]);

    notes.into_iter().for_each(|input_note_record| {
        let commit_height = input_note_record
            .inclusion_proof()
            .map(|proof| proof.origin().block_num.to_string())
            .unwrap_or("-".to_string());
        table.add_row(vec![
            input_note_record.note().id().inner().to_string(),
            input_note_record.note().script().hash().to_string(),
            input_note_record.note().assets().commitment().to_string(),
            input_note_record.note().inputs().commitment().to_string(),
            Digest::new(input_note_record.note().serial_num()).to_string(),
            commit_height,
        ]);
    });

    println!("{table}");
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use crate::cli::{
        get_note_with_id_prefix,
        input_notes::{export_note, import_note},
    };

    use miden_client::{
        config::{ClientConfig, Endpoint},
        errors::NoteIdPrefixFetchError,
        mock::{MockClient, MockDataStore, MockRpcApi},
        store::{sqlite_store::SqliteStore, InputNoteRecord},
    };
    use mock::mock::{
        account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs,
    };
    use std::env::temp_dir;
    use uuid::Uuid;

    #[tokio::test]
    async fn import_export_recorded_note() {
        // generate test client
        let mut path = temp_dir();
        path.push(Uuid::new_v4().to_string());
        let client_config = ClientConfig::new(
            path.into_os_string()
                .into_string()
                .unwrap()
                .try_into()
                .unwrap(),
            Endpoint::default().into(),
        );

        let store = SqliteStore::new((&client_config).into()).unwrap();

        let mut client = MockClient::new(
            MockRpcApi::new(&Endpoint::default().to_string()),
            store,
            MockDataStore::new(),
        )
        .unwrap();

        // generate test data
        let transaction_inputs = mock_inputs(
            MockAccountType::StandardExisting,
            AssetPreservationStatus::Preserved,
        );

        let committed_note: InputNoteRecord =
            transaction_inputs.input_notes().get_note(0).clone().into();
        let pending_note = InputNoteRecord::new(
            transaction_inputs.input_notes().get_note(1).note().clone(),
            None,
        );

        client.import_input_note(committed_note.clone()).unwrap();
        client.import_input_note(pending_note.clone()).unwrap();
        assert!(pending_note.inclusion_proof().is_none());
        assert!(committed_note.inclusion_proof().is_some());

        let mut filename_path = temp_dir();
        filename_path.push("test_import");

        let mut filename_path_pending = temp_dir();
        filename_path_pending.push("test_import_pending");

        export_note(
            &client,
            &committed_note.note_id().inner().to_string(),
            Some(filename_path.clone()),
        )
        .unwrap();

        assert!(filename_path.exists());

        export_note(
            &client,
            &pending_note.note_id().inner().to_string(),
            Some(filename_path_pending.clone()),
        )
        .unwrap();

        assert!(filename_path_pending.exists());

        // generate test client to import notes to
        let mut path = temp_dir();
        path.push(Uuid::new_v4().to_string());
        let client_config = ClientConfig::new(
            path.into_os_string()
                .into_string()
                .unwrap()
                .try_into()
                .unwrap(),
            Endpoint::default().into(),
        );
        let store = SqliteStore::new((&client_config).into()).unwrap();

        let mut client = MockClient::new(
            MockRpcApi::new(&Endpoint::default().to_string()),
            store,
            MockDataStore::new(),
        )
        .unwrap();

        import_note(&mut client, filename_path).unwrap();
        let imported_note_record: InputNoteRecord =
            client.get_input_note(committed_note.note().id()).unwrap();

        assert_eq!(committed_note.note().id(), imported_note_record.note().id());

        import_note(&mut client, filename_path_pending).unwrap();
        let imported_pending_note_record = client.get_input_note(pending_note.note().id()).unwrap();

        assert_eq!(
            imported_pending_note_record.note().id(),
            pending_note.note().id()
        );
    }

    #[tokio::test]
    async fn get_input_note_with_prefix() {
        // generate test client
        let mut path = temp_dir();
        path.push(Uuid::new_v4().to_string());
        let client_config = ClientConfig::new(
            path.into_os_string()
                .into_string()
                .unwrap()
                .try_into()
                .unwrap(),
            Endpoint::default().into(),
        );

        let store = SqliteStore::new((&client_config).into()).unwrap();

        let mut client = MockClient::new(
            MockRpcApi::new(&Endpoint::default().to_string()),
            store,
            MockDataStore::new(),
        )
        .unwrap();

        // Ensure we get an error if no note is found
        let non_existent_note_id = "0x123456";
        assert_eq!(
            get_note_with_id_prefix(&client, non_existent_note_id),
            Err(NoteIdPrefixFetchError::NoMatch(
                non_existent_note_id.to_string()
            ))
        );

        // generate test data
        let transaction_inputs = mock_inputs(
            MockAccountType::StandardExisting,
            AssetPreservationStatus::Preserved,
        );

        let committed_note: InputNoteRecord =
            transaction_inputs.input_notes().get_note(0).clone().into();
        let pending_note = InputNoteRecord::new(
            transaction_inputs.input_notes().get_note(1).note().clone(),
            None,
        );

        client.import_input_note(committed_note.clone()).unwrap();
        client.import_input_note(pending_note.clone()).unwrap();
        assert!(pending_note.inclusion_proof().is_none());
        assert!(committed_note.inclusion_proof().is_some());

        // Check that we can fetch Both notes
        let note = get_note_with_id_prefix(&client, &committed_note.note_id().to_hex()).unwrap();
        assert_eq!(note.note_id(), committed_note.note_id());

        let note = get_note_with_id_prefix(&client, &pending_note.note_id().to_hex()).unwrap();
        assert_eq!(note.note_id(), pending_note.note_id());

        // Check that we get an error if many match
        let note_id_with_many_matches = "0x";
        assert_eq!(
            get_note_with_id_prefix(&client, note_id_with_many_matches),
            Err(NoteIdPrefixFetchError::MultipleMatches(
                note_id_with_many_matches.to_string()
            ))
        );
    }
}

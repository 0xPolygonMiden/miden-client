use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use super::{Client, Parser};
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
use miden_client::store::notes::{InputNoteFilter, InputNoteRecord};

use crypto::utils::{Deserializable, Serializable};

use objects::notes::NoteId;
use objects::Digest;

#[derive(Debug, Parser, Clone)]
#[clap(about = "View input notes")]
pub enum InputNotes {
    /// List input notes
    #[clap(short_flag = 'l')]
    List,

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
    #[clap(short_flag = 's')]
    Export {
        /// Note ID of the input note to show
        #[clap()]
        id: String,

        /// Path to the file that will contain the input note data. If not provided, the filename will be the input note ID
        #[clap()]
        filename: Option<PathBuf>,
    },

    /// Import input note data from a binary file
    #[clap(short_flag = 's')]
    Import {
        /// Path to the file that contains the input note data
        #[clap()]
        filename: PathBuf,
    },
}

impl InputNotes {
    pub fn execute(&self, mut client: Client) -> Result<(), String> {
        match self {
            InputNotes::List => {
                list_input_notes(client)?;
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
            }
            InputNotes::Import { filename } => {
                import_note(&mut client, filename.clone())?;
            }
        }
        Ok(())
    }
}

// LIST INPUT NOTES
// ================================================================================================
fn list_input_notes(client: Client) -> Result<(), String> {
    let notes = client
        .get_input_notes(InputNoteFilter::All)
        .map_err(|err| err.to_string())?;
    print_notes_summary(&notes);
    Ok(())
}

// EXPORT INPUT NOTE
// ================================================================================================
pub fn export_note(
    client: &Client,
    note_id: &str,
    filename: Option<PathBuf>,
) -> Result<File, String> {
    let note_id = Digest::try_from(note_id)
        .map_err(|err| format!("Failed to parse input note id: {}", err))?
        .into();
    let note = client
        .get_input_note(note_id)
        .map_err(|err| err.to_string())?;

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
pub fn import_note(client: &mut Client, filename: PathBuf) -> Result<NoteId, String> {
    let mut contents = vec![];
    let mut _file = File::open(filename)
        .and_then(|mut f| f.read_to_end(&mut contents))
        .map_err(|err| err.to_string());

    // TODO: When importing a RecordedNote we want to make sure that the note actually exists in the chain (RPC call)
    // and start monitoring its nullifiers (ie, update the list of relevant tags in the state sync table)
    let input_note_record =
        InputNoteRecord::read_from_bytes(&contents).map_err(|err| err.to_string())?;

    let note_id = input_note_record.note().id();
    client
        .import_input_note(input_note_record)
        .map_err(|err| err.to_string())?;

    Ok(note_id)
}

// SHOW INPUT NOTE
// ================================================================================================
fn show_input_note(
    client: Client,
    note_id: String,
    show_script: bool,
    show_vault: bool,
    show_inputs: bool,
) -> Result<(), String> {
    let note_id = Digest::try_from(note_id)
        .map_err(|err| format!("Failed to parse input note with ID: {}", err))?
        .into();

    let input_note_record = client
        .get_input_note(note_id)
        .map_err(|err| err.to_string())?;

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
                Cell::new(input_note_record.note().inputs().hash()),
            ])
            .add_row(vec![Cell::new("Note Inputs").add_attribute(Attribute::Bold)]);
        input_note_record
            .note()
            .inputs()
            .inputs()
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
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec![
            Cell::new("note id").add_attribute(Attribute::Bold),
            Cell::new("script hash").add_attribute(Attribute::Bold),
            Cell::new("vault hash").add_attribute(Attribute::Bold),
            Cell::new("inputs hash").add_attribute(Attribute::Bold),
            Cell::new("serial num").add_attribute(Attribute::Bold),
        ]);

    notes.into_iter().for_each(|input_note_record| {
        table.add_row(vec![
            input_note_record.note().id().inner().to_string(),
            input_note_record.note().script().hash().to_string(),
            input_note_record.note().assets().commitment().to_string(),
            input_note_record.note().inputs().hash().to_string(),
            Digest::new(input_note_record.note().serial_num()).to_string(),
        ]);
    });

    println!("{table}");
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use crate::cli::input_notes::{export_note, import_note};

    use miden_client::{
        client::Client,
        config::{ClientConfig, Endpoint},
        store::notes::InputNoteFilter,
    };
    use std::env::temp_dir;
    use uuid::Uuid;

    #[tokio::test]
    pub async fn import_export_recorded_note() {
        // generate test client
        let mut path = temp_dir();
        path.push(Uuid::new_v4().to_string());
        let mut client = Client::new(ClientConfig::new(
            path.into_os_string().into_string().unwrap(),
            Endpoint::default(),
        ))
        .await
        .unwrap();

        // generate test data
        miden_client::mock::insert_mock_data(&mut client);

        let notes = client.get_input_notes(InputNoteFilter::All).unwrap();

        let input_note_record = notes.first().unwrap();

        let mut filename_path = temp_dir();
        filename_path.push("test_import");

        export_note(
            &client,
            &input_note_record.note().id().inner().to_string(),
            Some(filename_path.clone()),
        )
        .unwrap();

        assert!(filename_path.exists());

        let mut path = temp_dir();
        path.push(Uuid::new_v4().to_string());
        let mut client = Client::new(ClientConfig::new(
            path.into_os_string().into_string().unwrap(),
            Endpoint::default(),
        ))
        .await
        .unwrap();

        import_note(&mut client, filename_path).unwrap();
        let imported_note_record = client
            .get_input_note(input_note_record.note().id())
            .unwrap();

        assert_eq!(
            input_note_record.note().id(),
            imported_note_record.note().id()
        );

        // Import/export pending note
        // ------------------------------

        // generate test client
        let mut path = temp_dir();
        path.push(Uuid::new_v4().to_string());
        let mut client = Client::new(ClientConfig::new(
            path.into_os_string().into_string().unwrap(),
            Endpoint::default(),
        ))
        .await
        .unwrap();

        // generate test data
        miden_client::mock::insert_mock_data(&mut client);

        let pending_note = client.get_input_notes(InputNoteFilter::Pending).unwrap();
        let input_note_record = &pending_note.first().unwrap();
        assert!(input_note_record.inclusion_proof().is_none());

        let mut filename_path = temp_dir();
        filename_path.push("test_import_pending");
        export_note(
            &client,
            &input_note_record.note().id().inner().to_string(),
            Some(filename_path.clone()),
        )
        .unwrap();

        let mut path = temp_dir();
        path.push(Uuid::new_v4().to_string());
        let mut client = Client::new(ClientConfig::new(
            path.into_os_string().into_string().unwrap(),
            Endpoint::default(),
        ))
        .await
        .unwrap();

        import_note(&mut client, filename_path).unwrap();
        let imported_note = client
            .get_input_note(input_note_record.note().id())
            .unwrap();

        assert_eq!(input_note_record.note().id(), imported_note.note().id());
    }
}

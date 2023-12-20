use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use super::{Client, Parser};
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
use crypto::utils::{Deserializable, Serializable};
use miden_client::store::notes::InputNoteFilter;
use objects::notes::RecordedNote;
use objects::Digest;

#[derive(Debug, Parser, Clone)]
#[clap(about = "View input notes")]
pub enum InputNotes {
    /// List input notes
    #[clap(short_flag = 'l')]
    List,

    /// Show details of the input note for the specified note hash
    #[clap(short_flag = 's')]
    Show {
        /// Hash of the input note to show
        #[clap()]
        hash: String,

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
        /// Hash of the input note to show
        #[clap()]
        hash: String,

        /// Path to the file that will contain the input note data. If not provided, the filename will be the input note hash
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
    pub fn execute(&self, client: Client) -> Result<(), String> {
        match self {
            InputNotes::List => {
                list_input_notes(client)?;
            }
            InputNotes::Show {
                hash,
                script,
                vault,
                inputs,
            } => {
                show_input_note(client, hash.clone(), *script, *vault, *inputs)?;
            }
            InputNotes::Export { hash, filename } => {
                export_note(&client, hash, filename.clone())?;
            }
            InputNotes::Import { filename } => {
                import_note(client, filename.clone())?;
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
pub fn export_note(client: &Client, hash: &str, filename: Option<PathBuf>) -> Result<File, String> {
    let hash = Digest::try_from(hash)
        .map_err(|err| format!("Failed to parse input note hash: {}", err))?;
    let note = client.get_input_note(hash).map_err(|err| err.to_string())?;

    let file_path = filename.unwrap_or_else(|| {
        let mut dir = PathBuf::new();
        dir.push(hash.to_string());
        dir
    });

    let mut file = File::create(file_path).map_err(|err| err.to_string())?;

    let _ = file.write_all(&note.to_bytes());

    Ok(file)
}

// IMPORT INPUT NOTE
// ================================================================================================
pub fn import_note(mut client: Client, filename: PathBuf) -> Result<Digest, String> {
    let mut contents = vec![];
    let mut _file = File::open(filename)
        .and_then(|mut f| f.read_to_end(&mut contents))
        .map_err(|err| err.to_string());

    let note = RecordedNote::read_from_bytes(&contents).map_err(|err| err.to_string())?;

    client
        .insert_input_note(note.clone())
        .map_err(|err| err.to_string())?;
    Ok(note.note().hash())
}

// SHOW INPUT NOTE
// ================================================================================================
fn show_input_note(
    client: Client,
    hash: String,
    show_script: bool,
    show_vault: bool,
    show_inputs: bool,
) -> Result<(), String> {
    let hash = Digest::try_from(hash)
        .map_err(|err| format!("Failed to parse input note hash: {}", err))?;

    let note = client.get_input_note(hash).map_err(|err| err.to_string())?;

    // print note summary
    print_notes_summary(core::iter::once(&note));

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_HORIZONTAL_ONLY)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth);

    // print note script
    if show_script {
        table
            .add_row(vec![
                Cell::new("Note Script hash").add_attribute(Attribute::Bold),
                Cell::new(note.note().script().hash()),
            ])
            .add_row(vec![
                Cell::new("Note Script code").add_attribute(Attribute::Bold),
                Cell::new(note.note().script().code()),
            ]);
    };

    // print note vault
    if show_vault {
        table
            .add_row(vec![
                Cell::new("Note Vault hash").add_attribute(Attribute::Bold),
                Cell::new(note.note().vault().hash()),
            ])
            .add_row(vec![Cell::new("Note Vault").add_attribute(Attribute::Bold)]);

        note.note().vault().iter().for_each(|asset| {
            table.add_row(vec![Cell::new(format!("{:?}", asset))]);
        })
    };

    if show_inputs {
        table
            .add_row(vec![
                Cell::new("Note Inputs hash").add_attribute(Attribute::Bold),
                Cell::new(note.note().inputs().hash()),
            ])
            .add_row(vec![Cell::new("Note Inputs").add_attribute(Attribute::Bold)]);
        note.note()
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
    I: IntoIterator<Item = &'a RecordedNote>,
{
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec![
            Cell::new("hash").add_attribute(Attribute::Bold),
            Cell::new("script hash").add_attribute(Attribute::Bold),
            Cell::new("vault hash").add_attribute(Attribute::Bold),
            Cell::new("inputs hash").add_attribute(Attribute::Bold),
            Cell::new("serial num").add_attribute(Attribute::Bold),
        ]);

    notes.into_iter().for_each(|note| {
        table.add_row(vec![
            note.note().hash().to_string(),
            note.note().script().hash().to_string(),
            note.note().vault().hash().to_string(),
            note.note().inputs().hash().to_string(),
            Digest::new(note.note().serial_num()).to_string(),
        ]);
    });

    println!("{table}");
}

#[cfg(test)]
mod tests {
    use crate::cli::input_notes::{export_note, import_note};
    use miden_client::{
        client::Client,
        config::{ClientConfig, Endpoint},
        store::notes::InputNoteFilter,
    };
    use std::{env::temp_dir};
    use uuid::Uuid;

    #[tokio::test]
    pub async fn import_export() {
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

        let note = client
            .get_input_notes(InputNoteFilter::All)
            .unwrap()
            .first()
            .unwrap()
            .clone();

        let mut filename_path = temp_dir();
        filename_path.push("test_import");

        export_note(
            &client,
            &note.note().hash().to_string(),
            Some(filename_path.clone()),
        )
        .unwrap();

        assert!(filename_path.exists());

        let mut path = temp_dir();
        path.push(Uuid::new_v4().to_string());
        let client = Client::new(ClientConfig::new(
            path.into_os_string().into_string().unwrap(),
            Endpoint::default(),
        ))
        .await
        .unwrap();
        import_note(client, filename_path).unwrap();
    }
}

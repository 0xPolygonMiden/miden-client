use super::{Client, Parser};
use miden_client::InputNoteFilter;

use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
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

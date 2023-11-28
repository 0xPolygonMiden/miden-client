use super::Client;
use super::Parser;

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
    let notes = client.get_input_notes().map_err(|err| err.to_string())?;
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

    let note = client
        .store()
        .get_input_note_by_hash(hash)
        .map_err(|err| err.to_string())?;

    // print note summary
    print_notes_summary(core::iter::once(&note));

    // print note script
    if show_script {
        println!("{}", "-".repeat(240));
        println!("Note script hash: {}", note.note().script().hash());
        println!("{}", "-".repeat(240));
        println!("Note Script:");
        println!("{}", "-".repeat(240));
        println!("{}", note.note().script().code());
    };

    // print note vault
    if show_vault {
        println!("{}", "-".repeat(240));
        println!("Note vault hash: {}", note.note().vault().hash());
        println!("{}", "-".repeat(240));
        println!("Note Vault:");
        println!("{}", "-".repeat(240));
        for asset in note.note().vault().iter() {
            // To do print this nicely
            println!("{:?}", asset);
        }
    };

    if show_inputs {
        println!("{}", "-".repeat(240));
        println!("Note inputs hash: {}", note.note().inputs().hash());
        println!("{}", "-".repeat(240));
        println!("Note Inputs:");
        println!("{}", "-".repeat(240));
        for (idx, input) in note.note().inputs().inputs().iter().enumerate() {
            // To do print this nicely
            println!("{idx}: {input}");
        }
    };

    Ok(())
}

// HELPERS
// ================================================================================================
fn print_notes_summary<'a, I>(notes: I)
where
    I: IntoIterator<Item = &'a RecordedNote>,
{
    println!("{}", "-".repeat(240));
    println!(
        "{0: <66} | {1: <66} | {2: <66} | {3: <66} | {4: <15}",
        "hash", "script hash", "vault hash", "inputs hash", "serial num",
    );
    println!("{}", "-".repeat(240));

    for note in notes {
        println!(
            "{0: <66} | {1: <66} | {2: <66} | {3: <66} | {4: <15}",
            note.note().hash(),
            note.note().script().hash(),
            note.note().vault().hash(),
            note.note().inputs().hash(),
            Digest::new(note.note().serial_num()),
        );
    }
    println!("{}", "-".repeat(240));
}

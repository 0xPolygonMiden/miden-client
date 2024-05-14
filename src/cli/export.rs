use std::{fs::File, io::Write, path::PathBuf};

use miden_client::{
    client::{rpc::NodeRpcClient, Client},
    store::{InputNoteRecord, Store},
};
use miden_objects::{crypto::rand::FeltRng, Digest};
use miden_tx::{utils::Serializable, TransactionAuthenticator};

use super::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(about = "Export client objects")]
pub struct ExportCmd {
    /// ID of the output note to export
    #[clap()]
    id: String,

    /// Desired filename for the binary file. Defaults to the note ID if not provided
    #[clap(short, long, default_value = "false")]
    filename: Option<PathBuf>,
}

impl ExportCmd {
    pub fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        export_note(&client, self.id.as_str(), self.filename.clone())?;
        println!("Succesfully exported note {}", self.id.as_str());
        Ok(())
    }
}

// EXPORT NOTE
// ================================================================================================
pub fn export_note<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: &Client<N, R, S, A>,
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

    // Convert output note into InputNoteRecord before exporting
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

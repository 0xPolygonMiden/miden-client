use std::{fs::File, io::Write, path::PathBuf};

use miden_client::{
    auth::TransactionAuthenticator, crypto::FeltRng, notes::NoteFile, rpc::NodeRpcClient,
    store::Store, utils::Serializable, Client,
};
use tracing::info;

use super::Parser;
use crate::get_output_note_with_id_prefix;

#[derive(Debug, Parser, Clone)]
#[clap(about = "Export client output notes")]
pub struct ExportCmd {
    /// ID (or a valid prefix) of the output note to export
    #[clap()]
    id: String,

    /// Desired filename for the binary file. Defaults to the note ID if not provided
    #[clap(short, long)]
    filename: Option<PathBuf>,

    /// Exported note type
    #[clap(short, long, value_enum)]
    export_type: ExportType,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ExportType {
    Id,
    Full,
    Partial,
}

impl ExportCmd {
    pub fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        export_note(&client, self.id.as_str(), self.filename.clone(), self.export_type.clone())?;
        Ok(())
    }
}

// EXPORT NOTE
// ================================================================================================

pub fn export_note<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: &Client<N, R, S, A>,
    note_id: &str,
    filename: Option<PathBuf>,
    export_type: ExportType,
) -> Result<File, String> {
    let note_id = get_output_note_with_id_prefix(client, note_id)
        .map_err(|err| err.to_string())?
        .id();

    let output_note = client
        .get_output_notes(miden_client::store::NoteFilter::Unique(note_id))?
        .pop()
        .expect("should have an output note");

    let note_file = match export_type {
        ExportType::Id => NoteFile::NoteId(output_note.id()),
        ExportType::Full => match output_note.inclusion_proof() {
            Some(inclusion_proof) => {
                NoteFile::NoteWithProof(output_note.clone().try_into()?, inclusion_proof.clone())
            },
            None => return Err("Note does not have inclusion proofx".to_string()),
        },
        ExportType::Partial => NoteFile::NoteDetails(
            output_note.clone().try_into()?,
            Some(output_note.metadata().tag()),
        ),
    };

    let file_path = if let Some(filename) = filename {
        filename
    } else {
        let current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
        current_dir.join(format!("{}.mno", note_id.inner()))
    };

    info!("Writing file to {}", file_path.to_string_lossy());
    let mut file = File::create(file_path).map_err(|err| err.to_string())?;
    file.write_all(&note_file.to_bytes()).map_err(|err| err.to_string())?;

    println!("Succesfully exported note {}", note_id);
    Ok(file)
}

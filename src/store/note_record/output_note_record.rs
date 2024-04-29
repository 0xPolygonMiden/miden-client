use miden_objects::{
    notes::{Note, NoteAssets, NoteId, NoteInclusionProof, NoteMetadata},
    Digest,
};

use super::{InputNoteRecord, NoteRecordDetails, NoteStatus};
use crate::errors::ClientError;

// OUTPUT NOTE RECORD
// ================================================================================================

/// Represents a Note which was the result of executing some transaction of which the Store can
/// keep track and retrieve.
///
/// An [OutputNoteRecord] contains all the information of a [Note] while it allows for not knowing
/// the details (nullifier, script, inputs and serial number), in addition of (optionally) the
/// [NoteInclusionProof] that identifies when the note was included in the chain.
///
/// It is also possible to convert [Note] into [OutputNoteRecord] (we fill the `details` and
/// `inclusion_proof` fields if possible)
#[derive(Clone, Debug, PartialEq)]
pub struct OutputNoteRecord {
    assets: NoteAssets,
    details: Option<NoteRecordDetails>,
    id: NoteId,
    inclusion_proof: Option<NoteInclusionProof>,
    metadata: NoteMetadata,
    recipient: Digest,
    status: NoteStatus,
}

impl OutputNoteRecord {
    pub fn new(
        id: NoteId,
        recipient: Digest,
        assets: NoteAssets,
        status: NoteStatus,
        metadata: NoteMetadata,
        inclusion_proof: Option<NoteInclusionProof>,
        details: Option<NoteRecordDetails>,
    ) -> OutputNoteRecord {
        OutputNoteRecord {
            id,
            recipient,
            assets,
            status,
            metadata,
            inclusion_proof,
            details,
        }
    }

    pub fn id(&self) -> NoteId {
        self.id
    }

    pub fn recipient(&self) -> Digest {
        self.recipient
    }

    pub fn assets(&self) -> &NoteAssets {
        &self.assets
    }

    pub fn status(&self) -> NoteStatus {
        self.status
    }

    pub fn metadata(&self) -> &NoteMetadata {
        &self.metadata
    }

    pub fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        self.inclusion_proof.as_ref()
    }

    pub fn details(&self) -> Option<&NoteRecordDetails> {
        self.details.as_ref()
    }
}

impl From<Note> for OutputNoteRecord {
    fn from(note: Note) -> Self {
        OutputNoteRecord {
            id: note.id(),
            recipient: note.recipient_digest(),
            assets: note.assets().clone(),
            status: NoteStatus::Pending,
            metadata: *note.metadata(),
            inclusion_proof: None,
            details: Some(NoteRecordDetails::new(
                note.nullifier().to_string(),
                note.script().clone(),
                note.inputs().to_vec(),
                note.serial_num(),
            )),
        }
    }
}

impl TryFrom<InputNoteRecord> for OutputNoteRecord {
    type Error = ClientError;

    fn try_from(input_note: InputNoteRecord) -> Result<Self, Self::Error> {
        match input_note.metadata() {
            Some(metadata) => Ok(OutputNoteRecord {
                assets: input_note.assets().clone(),
                details: Some(input_note.details().clone()),
                id: input_note.id(),
                inclusion_proof: input_note.inclusion_proof().cloned(),
                metadata: *metadata,
                recipient: input_note.recipient(),
                status: input_note.status(),
            }),
            None => Err(ClientError::NoteError(miden_objects::NoteError::invalid_origin_index(
                "Input Note Record contains no metadata".to_string(),
            ))),
        }
    }
}

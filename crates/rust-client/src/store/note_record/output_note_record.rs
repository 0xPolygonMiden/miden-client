use alloc::string::ToString;

use miden_objects::{
    notes::{
        Note, NoteAssets, NoteDetails, NoteId, NoteInclusionProof, NoteInputs, NoteMetadata,
        NoteRecipient, PartialNote,
    },
    transaction::OutputNote,
    Digest,
};

use super::{InputNoteRecord, NoteRecordDetails, NoteStatus};
use crate::ClientError;

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
///
/// The `consumer_account_id` field is used to keep track of the account that consumed the note. It
/// is only valid if the `status` is [NoteStatus::Consumed]. If the note is consumed but the field
/// is [None] it means that the note was consumed by an untracked account.
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

// CONVERSIONS
// ================================================================================================

// TODO: Improve conversions by implementing into_parts()
impl From<Note> for OutputNoteRecord {
    fn from(note: Note) -> Self {
        OutputNoteRecord {
            id: note.id(),
            recipient: note.recipient().digest(),
            assets: note.assets().clone(),
            status: NoteStatus::Expected { created_at: 0 },
            metadata: *note.metadata(),
            inclusion_proof: None,
            details: Some(NoteRecordDetails::new(
                note.nullifier().to_string(),
                note.script().clone(),
                note.inputs().values().to_vec(),
                note.serial_num(),
            )),
        }
    }
}

impl From<PartialNote> for OutputNoteRecord {
    fn from(partial_note: PartialNote) -> Self {
        OutputNoteRecord::new(
            partial_note.id(),
            partial_note.recipient_digest(),
            partial_note.assets().clone(),
            NoteStatus::Expected { created_at: 0 },
            *partial_note.metadata(),
            None,
            None,
        )
    }
}

/// [OutputNote] can always be turned into an [OutputNoteRecord] when they're either
/// [OutputNote::Full] or [OutputNote::Partial] and always fail the conversion if it's
/// [OutputNote::Header]. This also mean that `output_note.try_from()` can also be used as a way to
/// filter the full and partial output notes
impl TryFrom<OutputNote> for OutputNoteRecord {
    type Error = ClientError;

    fn try_from(output_note: OutputNote) -> Result<Self, Self::Error> {
        match output_note {
            OutputNote::Full(note) => Ok(note.into()),
            OutputNote::Partial(partial_note)=> {
                Ok(partial_note.into())
            },
            OutputNote::Header(_) => Err(ClientError::NoteRecordError("Cannot transform a Header output note into an OutputNoteRecord: not enough information".to_string()))
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

impl TryFrom<OutputNoteRecord> for NoteDetails {
    type Error = ClientError;
    fn try_from(value: OutputNoteRecord) -> Result<Self, Self::Error> {
        match value.details() {
            Some(details) => Ok(NoteDetails::new(
                value.assets.clone(),
                NoteRecipient::new(
                    details.serial_num,
                    details.script.clone(),
                    NoteInputs::new(details.inputs.clone())?,
                ),
            )),
            None => Err(ClientError::NoteRecordError(
                "Output Note Record contains no details".to_string(),
            )),
        }
    }
}

impl TryFrom<OutputNoteRecord> for Note {
    type Error = ClientError;

    fn try_from(value: OutputNoteRecord) -> Result<Self, Self::Error> {
        match value.details {
            Some(details) => {
                let note_inputs = NoteInputs::new(details.inputs)?;
                let note_recipient =
                    NoteRecipient::new(details.serial_num, details.script, note_inputs);
                let note = Note::new(value.assets, value.metadata, note_recipient);
                Ok(note)
            },
            None => Err(ClientError::NoteRecordError(
                "Output Note Record contains no details".to_string(),
            )),
        }
    }
}

use miden_objects::{
    notes::{
        Note, NoteAssets, NoteId, NoteInclusionProof, NoteInputs, NoteMetadata, NoteRecipient,
    },
    transaction::InputNote,
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest,
};

use super::{NoteRecordDetails, NoteStatus, OutputNoteRecord};
use crate::errors::ClientError;

// INPUT NOTE RECORD
// ================================================================================================

/// Represents a Note of which the Store can keep track and retrieve.
///
/// An [InputNoteRecord] contains all the information of a [Note], in addition of (optionally) the
/// [NoteInclusionProof] that identifies when the note was included in the chain.
///
/// Once the proof is set, the [InputNoteRecord] can be transformed into an [InputNote] and used as
/// input for transactions.
///
/// It is also possible to convert [Note] and [InputNote] into [InputNoteRecord] (we fill the
/// `metadata` and `inclusion_proof` fields if possible)
#[derive(Clone, Debug, PartialEq)]
pub struct InputNoteRecord {
    assets: NoteAssets,
    details: NoteRecordDetails,
    id: NoteId,
    inclusion_proof: Option<NoteInclusionProof>,
    metadata: Option<NoteMetadata>,
    recipient: Digest,
    status: NoteStatus,
}

impl InputNoteRecord {
    pub fn new(
        id: NoteId,
        recipient: Digest,
        assets: NoteAssets,
        status: NoteStatus,
        metadata: Option<NoteMetadata>,
        inclusion_proof: Option<NoteInclusionProof>,
        details: NoteRecordDetails,
    ) -> InputNoteRecord {
        InputNoteRecord {
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

    pub fn metadata(&self) -> Option<&NoteMetadata> {
        self.metadata.as_ref()
    }

    pub fn nullifier(&self) -> &str {
        &self.details.nullifier
    }

    pub fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        self.inclusion_proof.as_ref()
    }

    pub fn details(&self) -> &NoteRecordDetails {
        &self.details
    }
}

impl Serializable for InputNoteRecord {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.id().write_into(target);
        self.recipient().write_into(target);
        self.assets().write_into(target);
        self.status().write_into(target);
        self.metadata().write_into(target);
        self.details().write_into(target);
        self.inclusion_proof().write_into(target);
    }
}

impl Deserializable for InputNoteRecord {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let id = NoteId::read_from(source)?;
        let recipient = Digest::read_from(source)?;
        let assets = NoteAssets::read_from(source)?;
        let status = NoteStatus::read_from(source)?;
        let metadata = Option::<NoteMetadata>::read_from(source)?;
        let details = NoteRecordDetails::read_from(source)?;
        let inclusion_proof = Option::<NoteInclusionProof>::read_from(source)?;

        Ok(InputNoteRecord {
            id,
            recipient,
            assets,
            status,
            metadata,
            inclusion_proof,
            details,
        })
    }
}

impl From<Note> for InputNoteRecord {
    fn from(note: Note) -> Self {
        InputNoteRecord {
            id: note.id(),
            recipient: note.recipient_digest(),
            assets: note.assets().clone(),
            status: NoteStatus::Pending,
            metadata: Some(*note.metadata()),
            inclusion_proof: None,
            details: NoteRecordDetails::new(
                note.nullifier().to_string(),
                note.script().clone(),
                note.inputs().to_vec(),
                note.serial_num(),
            ),
        }
    }
}

impl From<InputNote> for InputNoteRecord {
    fn from(recorded_note: InputNote) -> Self {
        InputNoteRecord {
            id: recorded_note.note().id(),
            recipient: recorded_note.note().recipient_digest(),
            assets: recorded_note.note().assets().clone(),
            status: NoteStatus::Pending,
            metadata: Some(*recorded_note.note().metadata()),
            details: NoteRecordDetails::new(
                recorded_note.note().nullifier().to_string(),
                recorded_note.note().script().clone(),
                recorded_note.note().inputs().values().to_vec(),
                recorded_note.note().serial_num(),
            ),
            inclusion_proof: Some(recorded_note.proof().clone()),
        }
    }
}

impl TryInto<InputNote> for InputNoteRecord {
    type Error = ClientError;

    fn try_into(self) -> Result<InputNote, Self::Error> {
        match (self.inclusion_proof, self.metadata) {
            (Some(proof), Some(metadata)) => {
                // TODO: Write functions to get these fields more easily
                let note_inputs = NoteInputs::new(self.details.inputs)?;
                let note_recipient =
                    NoteRecipient::new(self.details.serial_num, self.details.script, note_inputs);
                let note = Note::new(self.assets, metadata, note_recipient);
                Ok(InputNote::new(note, proof.clone()))
            },

            (None, _) => Err(ClientError::NoteRecordError(
                "Input Note Record contains no inclusion proof".to_string(),
            )),
            (_, None) => Err(ClientError::NoteRecordError(
                "Input Note Record contains no metadata".to_string(),
            )),
        }
    }
}

impl TryInto<Note> for InputNoteRecord {
    type Error = ClientError;

    fn try_into(self) -> Result<Note, Self::Error> {
        match self.metadata {
            Some(metadata) => {
                let note_inputs = NoteInputs::new(self.details.inputs)?;
                let note_recipient =
                    NoteRecipient::new(self.details.serial_num, self.details.script, note_inputs);
                let note = Note::new(self.assets, metadata, note_recipient);
                Ok(note)
            },
            None => Err(ClientError::NoteRecordError(
                "Input Note Record contains no metadata".to_string(),
            )),
        }
    }
}

impl TryFrom<OutputNoteRecord> for InputNoteRecord {
    type Error = ClientError;

    fn try_from(output_note: OutputNoteRecord) -> Result<Self, Self::Error> {
        match output_note.details() {
            Some(details) => Ok(InputNoteRecord {
                assets: output_note.assets().clone(),
                details: details.clone(),
                id: output_note.id(),
                inclusion_proof: output_note.inclusion_proof().cloned(),
                metadata: Some(*output_note.metadata()),
                recipient: output_note.recipient(),
                status: output_note.status(),
            }),
            None => Err(ClientError::NoteError(miden_objects::NoteError::invalid_origin_index(
                "Output Note Record contains no details".to_string(),
            ))),
        }
    }
}

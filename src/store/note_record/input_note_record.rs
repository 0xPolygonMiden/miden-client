use super::{NoteRecordDetails, NoteStatus};
use crate::errors::ClientError;
use miden_objects::{
    notes::{Note, NoteAssets, NoteId, NoteInclusionProof, NoteInputs, NoteMetadata, NoteScript},
    transaction::InputNote,
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest, NoteError,
};

// INPUT NOTE RECORD
// ================================================================================================

/// Represents a Note of which the [Store] can keep track and retrieve.
///
/// An [InputNoteRecord] contains all the information of a [Note], in addition of (optionally)
/// the [NoteInclusionProof] that identifies when the note was included in the chain. Once the
/// proof is set, the [InputNoteRecord] can be transformed into an [InputNote] and used as input
/// for transactions.
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
            recipient: note.recipient(),
            assets: note.assets().clone(),
            status: NoteStatus::Pending,
            metadata: Some(*note.metadata()),
            inclusion_proof: None,
            details: NoteRecordDetails {
                nullifier: note.nullifier().to_string(),
                script: note.script().to_bytes(),
                inputs: note.inputs().to_bytes(),
                serial_num: note.serial_num(),
            },
        }
    }
}

impl From<InputNote> for InputNoteRecord {
    fn from(recorded_note: InputNote) -> Self {
        InputNoteRecord {
            id: recorded_note.note().id(),
            recipient: recorded_note.note().recipient(),
            assets: recorded_note.note().assets().clone(),
            status: NoteStatus::Pending,
            metadata: Some(*recorded_note.note().metadata()),
            details: NoteRecordDetails {
                nullifier: recorded_note.note().nullifier().to_string(),
                script: recorded_note.note().script().to_bytes(),
                inputs: recorded_note.note().inputs().to_bytes(),
                serial_num: recorded_note.note().serial_num(),
            },
            inclusion_proof: Some(recorded_note.proof().clone()),
        }
    }
}

impl TryInto<InputNote> for InputNoteRecord {
    type Error = ClientError;

    fn try_into(self) -> Result<InputNote, Self::Error> {
        match (self.inclusion_proof, self.metadata) {
            (Some(proof), Some(metadata)) => {
                let script = NoteScript::read_from_bytes(&self.details.script).map_err(|err| {
                    ClientError::NoteError(NoteError::NoteDeserializationError(err))
                })?;
                let inputs = NoteInputs::read_from_bytes(&self.details.inputs).map_err(|err| {
                    ClientError::NoteError(NoteError::NoteDeserializationError(err))
                })?;
                let note = Note::from_parts(
                    script,
                    inputs,
                    self.assets,
                    self.details.serial_num,
                    metadata,
                );
                Ok(InputNote::new(note, proof.clone()))
            }

            (None, _) => Err(ClientError::NoteError(
                miden_objects::NoteError::invalid_origin_index(
                    "Input Note Record contains no inclusion proof".to_string(),
                ),
            )),
            (_, None) => Err(ClientError::NoteError(
                miden_objects::NoteError::invalid_origin_index(
                    "Input Note Record contains no metadata".to_string(),
                ),
            )),
        }
    }
}

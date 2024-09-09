use alloc::string::ToString;

use miden_objects::{
    notes::{Note, NoteAssets, NoteDetails, NoteId, NoteInclusionProof, NoteMetadata, Nullifier},
    transaction::InputNote,
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest,
};

use super::NoteState;
use crate::ClientError;

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
/// The `consumer_account_id` field is used to keep track of the account that consumed the note. It
/// is only valid if the `status` is [NoteStatus::Consumed]. If the note is consumed but the field
/// is [None] it means that the note was consumed by an untracked account.
///
/// It is also possible to convert [Note] and [InputNote] into [InputNoteRecord] (we fill the
/// `metadata` and `inclusion_proof` fields if possible)
#[derive(Clone, Debug, PartialEq)]
pub struct InputNoteRecord {
    details: NoteDetails,
    created_at: Option<u64>,
    state: NoteState,
}

impl InputNoteRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(details: NoteDetails, created_at: Option<u64>, state: NoteState) -> InputNoteRecord {
        InputNoteRecord { details, created_at, state }
    }

    pub fn id(&self) -> NoteId {
        self.details.id()
    }

    pub fn recipient(&self) -> Digest {
        self.details.recipient().digest()
    }

    pub fn assets(&self) -> &NoteAssets {
        &self.details.assets()
    }

    pub fn state(&self) -> &NoteState {
        &self.state
    }

    pub fn metadata(&self) -> Option<&NoteMetadata> {
        match &self.state {
            NoteState::Committed { metadata, .. }
            | NoteState::Unverified { metadata, .. }
            | NoteState::Invalid { metadata, .. }
            | NoteState::ProcessingAuthenticated { metadata, .. }
            | NoteState::NativeConsumed { metadata, .. } => Some(metadata),
            _ => None,
        }
    }

    pub fn nullifier(&self) -> Nullifier {
        self.details.nullifier()
    }

    pub fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        match &self.state {
            NoteState::Committed { inclusion_proof, .. }
            | NoteState::Unverified { inclusion_proof, .. }
            | NoteState::Invalid {
                invalid_inclusion_proof: inclusion_proof, ..
            }
            | NoteState::ProcessingAuthenticated { inclusion_proof, .. }
            | NoteState::NativeConsumed { inclusion_proof, .. } => Some(inclusion_proof),
            _ => None,
        }
    }

    pub fn details(&self) -> &NoteDetails {
        &self.details
    }

    /// Returns whether the note record contains a valid inclusion proof correlated with its
    /// status
    pub fn is_authenticated(&self) -> bool {
        match self.state {
            NoteState::Committed { .. }
            | NoteState::ProcessingAuthenticated { .. }
            | NoteState::NativeConsumed { .. } => true,
            _ => false,
        }
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for InputNoteRecord {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.details.write_into(target);
        self.created_at.write_into(target);
        self.state.write_into(target);
    }
}

impl Deserializable for InputNoteRecord {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let details = NoteDetails::read_from(source)?;
        let created_at = Option::<u64>::read_from(source)?;
        let state = NoteState::read_from(source)?;

        Ok(InputNoteRecord { details, created_at, state })
    }
}

// CONVERSION
// ================================================================================================
impl From<&NoteDetails> for InputNoteRecord {
    fn from(value: &NoteDetails) -> Self {
        Self {
            details: value.clone(),
            created_at: None,
            state: NoteState::Unknown,
        }
    }
}

impl From<Note> for InputNoteRecord {
    fn from(value: Note) -> Self {
        Self {
            details: value.into(),
            created_at: None,
            state: NoteState::Unknown,
        }
    }
}

impl From<InputNote> for InputNoteRecord {
    fn from(value: InputNote) -> Self {
        match value {
            InputNote::Authenticated { note, proof } => Self {
                details: note.clone().into(),
                created_at: None,
                state: NoteState::Unverified {
                    metadata: note.metadata().clone(),
                    inclusion_proof: proof,
                },
            },
            InputNote::Unauthenticated { note } => note.into(),
        }
    }
}

impl TryInto<InputNote> for InputNoteRecord {
    type Error = ClientError;

    fn try_into(self) -> Result<InputNote, Self::Error> {
        match self.state {
            NoteState::Committed { inclusion_proof, metadata, .. }
            | NoteState::ProcessingAuthenticated { inclusion_proof, metadata, .. }
            | NoteState::NativeConsumed { inclusion_proof, metadata, .. } => {
                let note = Note::new(
                    self.details.assets().clone(),
                    metadata,
                    self.details.recipient().clone(),
                );

                Ok(InputNote::authenticated(note, inclusion_proof))
            },
            NoteState::Unverified { .. } => Err(ClientError::NoteRecordError(
                "Input Note Record proof is unverified".to_string(),
            )),
            NoteState::Invalid { .. } => {
                Err(ClientError::NoteRecordError("Input Note Record proof is invalid".to_string()))
            },
            _ => Err(ClientError::NoteRecordError(
                "Input Note Record contains no metadata".to_string(),
            )),
        }
    }
}

impl TryInto<Note> for InputNoteRecord {
    type Error = ClientError;

    fn try_into(self) -> Result<Note, Self::Error> {
        match self.metadata().cloned() {
            Some(metadata) => Ok(Note::new(
                self.details.assets().clone(),
                metadata,
                self.details.recipient().clone(),
            )),
            None => Err(ClientError::NoteRecordError(
                "Input Note Record contains no metadata".to_string(),
            )),
        }
    }
}

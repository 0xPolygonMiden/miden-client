use alloc::string::ToString;
use core::fmt::{self, Display};

use miden_objects::{
    notes::{
        Note, NoteAssets, NoteDetails, NoteId, NoteInclusionProof, NoteMetadata, NoteRecipient,
        Nullifier, PartialNote,
    },
    transaction::OutputNote,
    Digest,
};
use miden_tx::utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable};

use super::NoteRecordError;

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
/// is only valid if the `status` is [OutputNoteState::Consumed]. If the note is consumed but the
/// field is [None] it means that the note was consumed by an untracked account.
#[derive(Clone, Debug, PartialEq)]
pub struct OutputNoteRecord {
    assets: NoteAssets,
    id: NoteId,
    metadata: NoteMetadata,
    recipient_digest: Digest,
    state: OutputNoteState,
}

impl OutputNoteRecord {
    pub fn new(
        id: NoteId,
        recipient_digest: Digest,
        assets: NoteAssets,
        metadata: NoteMetadata,
        state: OutputNoteState,
    ) -> OutputNoteRecord {
        OutputNoteRecord {
            id,
            recipient_digest,
            assets,
            state,
            metadata,
        }
    }

    pub fn id(&self) -> NoteId {
        self.id
    }

    pub fn recipient_digest(&self) -> Digest {
        self.recipient_digest
    }

    pub fn assets(&self) -> &NoteAssets {
        &self.assets
    }

    pub fn state(&self) -> &OutputNoteState {
        &self.state
    }

    pub fn metadata(&self) -> &NoteMetadata {
        &self.metadata
    }

    pub fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        self.state.inclusion_proof()
    }

    pub fn recipient(&self) -> Option<&NoteRecipient> {
        self.state.recipient()
    }

    pub fn nullifier(&self) -> Option<Nullifier> {
        let recipient = self.recipient()?;
        Some(Nullifier::new(
            recipient.script().hash(),
            recipient.inputs().commitment(),
            self.assets.commitment(),
            recipient.serial_num(),
        ))
    }

    pub fn inclusion_proof_received(
        &mut self,
        inclusion_proof: NoteInclusionProof,
    ) -> Result<bool, NoteRecordError> {
        let new_state = self.state.inclusion_proof_received(inclusion_proof)?;
        if let Some(new_state) = new_state {
            self.state = new_state;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn nullifier_received(
        &mut self,
        nullifier: Nullifier,
        block_height: u32,
    ) -> Result<bool, NoteRecordError> {
        if let Some(note_nullifier) = self.nullifier() {
            if note_nullifier != nullifier {
                return Err(NoteRecordError::StateTransitionError(
                    "Nullifier does not match the expected value".to_string(),
                ));
            }
        }

        let new_state = self.state.nullifier_received(block_height)?;
        if let Some(new_state) = new_state {
            self.state = new_state;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

// CONVERSIONS
// ================================================================================================

// TODO: Improve conversions by implementing into_parts()
impl From<Note> for OutputNoteRecord {
    fn from(note: Note) -> Self {
        let header = note.header().clone();
        let (assets, recipient) = NoteDetails::from(note).into_parts();
        OutputNoteRecord {
            id: header.id(),
            recipient_digest: recipient.digest(),
            assets,
            metadata: *header.metadata(),
            state: OutputNoteState::ExpectedFull { block_height: None, recipient },
        }
    }
}

impl From<PartialNote> for OutputNoteRecord {
    fn from(partial_note: PartialNote) -> Self {
        OutputNoteRecord {
            id: partial_note.id(),
            recipient_digest: partial_note.recipient_digest(),
            assets: partial_note.assets().clone(),
            metadata: *partial_note.metadata(),
            state: OutputNoteState::ExpectedPartial { block_height: None },
        }
    }
}

/// [OutputNote] can always be turned into an [OutputNoteRecord] when they're either
/// [OutputNote::Full] or [OutputNote::Partial] and always fail the conversion if it's
/// [OutputNote::Header]. This also mean that `output_note.try_from()` can also be used as a way to
/// filter the full and partial output notes
impl TryFrom<OutputNote> for OutputNoteRecord {
    type Error = NoteRecordError;

    fn try_from(output_note: OutputNote) -> Result<Self, Self::Error> {
        match output_note {
            OutputNote::Full(note) => Ok(note.into()),
            OutputNote::Partial(partial_note)=> {
                Ok(partial_note.into())
            },
            OutputNote::Header(_) => Err(NoteRecordError::ConversionError("Cannot transform a Header output note into an OutputNoteRecord: not enough information".to_string()))
        }
    }
}

impl TryFrom<OutputNoteRecord> for NoteDetails {
    type Error = NoteRecordError;
    fn try_from(value: OutputNoteRecord) -> Result<Self, Self::Error> {
        match value.recipient() {
            Some(recipient) => Ok(NoteDetails::new(value.assets.clone(), recipient.clone())),
            None => Err(NoteRecordError::ConversionError(
                "Output Note Record contains no details".to_string(),
            )),
        }
    }
}

impl TryFrom<OutputNoteRecord> for Note {
    type Error = NoteRecordError;

    fn try_from(value: OutputNoteRecord) -> Result<Self, Self::Error> {
        match value.recipient() {
            Some(recipient) => {
                let note = Note::new(value.assets.clone(), value.metadata, recipient.clone());
                Ok(note)
            },
            None => Err(NoteRecordError::ConversionError(
                "Output Note Record contains no details".to_string(),
            )),
        }
    }
}

// OUTPUT NOTE STATE
// ================================================================================================

pub const STATE_EXPECTED_PARTIAL: u8 = 0;
pub const STATE_EXPECTED_FULL: u8 = 1;
pub const STATE_COMMITTED_PARTIAL: u8 = 2;
pub const STATE_COMMITTED_FULL: u8 = 3;
pub const STATE_CONSUMED: u8 = 4;

/// Possible states for a single output note. They describe the note's state and dictate its
/// lifecycle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputNoteState {
    /// Note without known recipient is expected to be committed on chain.
    ExpectedPartial {
        /// Block height at which the note is expected to be committed. If the block height is not
        /// known, this field will be `None`.
        block_height: Option<u32>,
    },
    /// Note with known recipient is expected to be committed on chain.
    ExpectedFull {
        /// Block height at which the note is expected to be committed. If the block height is not
        /// known, this field will be `None`.
        block_height: Option<u32>,
        /// Details needed consume the note.
        recipient: NoteRecipient,
    },
    /// Note without known recipient has been committed on chain, and can be consumed in a
    /// transaction.
    CommittedPartial {
        /// Inclusion proof for the note inside the chain block.
        inclusion_proof: NoteInclusionProof,
    },
    /// Note with known recipient has been committed on chain, and can be consumed in a
    /// transaction.
    CommittedFull {
        /// Details needed to consume the note.
        recipient: NoteRecipient,
        /// Inclusion proof for the note inside the chain block.
        inclusion_proof: NoteInclusionProof,
    },
    /// Note has been nullified on chain.
    Consumed {
        /// Block height at which the note was consumed.
        block_height: u32,
        /// Details needed to consume the note.
        recipient: NoteRecipient,
    },
}

impl OutputNoteState {
    /// Returns a unique identifier for each note state.
    pub fn discriminant(&self) -> u8 {
        match self {
            OutputNoteState::ExpectedPartial { .. } => STATE_EXPECTED_PARTIAL,
            OutputNoteState::ExpectedFull { .. } => STATE_EXPECTED_FULL,
            OutputNoteState::CommittedPartial { .. } => STATE_COMMITTED_PARTIAL,
            OutputNoteState::CommittedFull { .. } => STATE_COMMITTED_FULL,
            OutputNoteState::Consumed { .. } => STATE_CONSUMED,
        }
    }

    pub fn recipient(&self) -> Option<&NoteRecipient> {
        match self {
            OutputNoteState::ExpectedFull { recipient, .. } => Some(recipient),
            OutputNoteState::CommittedFull { recipient, .. } => Some(recipient),
            OutputNoteState::Consumed { recipient, .. } => Some(recipient),
            _ => None,
        }
    }

    pub fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        match self {
            OutputNoteState::CommittedPartial { inclusion_proof, .. }
            | OutputNoteState::CommittedFull { inclusion_proof, .. } => Some(inclusion_proof),
            _ => None,
        }
    }

    pub fn inclusion_proof_received(
        &self,
        inclusion_proof: NoteInclusionProof,
    ) -> Result<Option<OutputNoteState>, NoteRecordError> {
        match self {
            OutputNoteState::ExpectedPartial { .. } => {
                Ok(Some(OutputNoteState::CommittedPartial { inclusion_proof }))
            },
            OutputNoteState::ExpectedFull { recipient, .. } => {
                Ok(Some(OutputNoteState::CommittedFull {
                    recipient: recipient.clone(),
                    inclusion_proof,
                }))
            },
            OutputNoteState::CommittedPartial { inclusion_proof: prev_inclusion_proof }
            | OutputNoteState::CommittedFull {
                inclusion_proof: prev_inclusion_proof, ..
            } => {
                if prev_inclusion_proof == &inclusion_proof {
                    Ok(None)
                } else {
                    Err(NoteRecordError::StateTransitionError(
                        "Cannot receive different inclusion proof for committed note".to_string(),
                    ))
                }
            },
            OutputNoteState::Consumed { .. } => Ok(None),
        }
    }

    pub fn nullifier_received(
        &self,
        block_height: u32,
    ) -> Result<Option<OutputNoteState>, NoteRecordError> {
        match self {
            OutputNoteState::Consumed { .. } => Ok(None),
            OutputNoteState::ExpectedFull { recipient, .. }
            | OutputNoteState::CommittedFull { recipient, .. } => {
                Ok(Some(OutputNoteState::Consumed {
                    block_height,
                    recipient: recipient.clone(),
                }))
            },
            OutputNoteState::ExpectedPartial { .. } | OutputNoteState::CommittedPartial { .. } => {
                Err(NoteRecordError::InvalidStateTransition(
                    "Cannot nullify note without recipient".to_string(),
                ))
            },
        }
    }
}

impl Serializable for OutputNoteState {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.discriminant().write_into(target);
        match self {
            OutputNoteState::ExpectedPartial { block_height } => {
                block_height.write_into(target);
            },
            OutputNoteState::ExpectedFull { block_height, recipient } => {
                block_height.write_into(target);
                recipient.write_into(target);
            },
            OutputNoteState::CommittedPartial { inclusion_proof } => {
                inclusion_proof.write_into(target);
            },
            OutputNoteState::CommittedFull { recipient, inclusion_proof } => {
                recipient.write_into(target);
                inclusion_proof.write_into(target);
            },
            OutputNoteState::Consumed { block_height, recipient } => {
                block_height.write_into(target);
                recipient.write_into(target);
            },
        }
    }
}

impl Deserializable for OutputNoteState {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let status = source.read_u8()?;
        match status {
            STATE_EXPECTED_PARTIAL => {
                let block_height = source.read_u32()?;
                Ok(OutputNoteState::ExpectedPartial { block_height: Some(block_height) })
            },
            STATE_EXPECTED_FULL => {
                let block_height = source.read_u32()?;
                let recipient = NoteRecipient::read_from(source)?;
                Ok(OutputNoteState::ExpectedFull {
                    block_height: Some(block_height),
                    recipient,
                })
            },
            STATE_COMMITTED_PARTIAL => {
                let inclusion_proof = NoteInclusionProof::read_from(source)?;
                Ok(OutputNoteState::CommittedPartial { inclusion_proof })
            },
            STATE_COMMITTED_FULL => {
                let recipient = NoteRecipient::read_from(source)?;
                let inclusion_proof = NoteInclusionProof::read_from(source)?;
                Ok(OutputNoteState::CommittedFull { recipient, inclusion_proof })
            },
            STATE_CONSUMED => {
                let block_height = source.read_u32()?;
                let recipient = NoteRecipient::read_from(source)?;
                Ok(OutputNoteState::Consumed { block_height, recipient })
            },
            _ => Err(DeserializationError::InvalidValue("OutputNoteState".to_string())),
        }
    }
}

impl Display for OutputNoteState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputNoteState::ExpectedPartial => {
                write!(f, "Expected Partial")
            },
            OutputNoteState::ExpectedFull { .. } => {
                write!(f, "Expected Full")
            },
            OutputNoteState::CommittedPartial { inclusion_proof } => {
                write!(
                    f,
                    "Committed Partial (at block height {})",
                    inclusion_proof.location().block_num()
                )
            },
            OutputNoteState::CommittedFull { inclusion_proof, .. } => {
                write!(
                    f,
                    "Committed Full (at block height {})",
                    inclusion_proof.location().block_num()
                )
            },
            OutputNoteState::Consumed { block_height, .. } => {
                write!(f, "Consumed (at block height {block_height})")
            },
        }
    }
}

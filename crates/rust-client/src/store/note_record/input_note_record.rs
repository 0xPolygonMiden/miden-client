use alloc::string::ToString;

use miden_objects::{
    accounts::AccountId,
    notes::{
        compute_note_hash, Note, NoteAssets, NoteDetails, NoteId, NoteInclusionProof, NoteMetadata,
        Nullifier,
    },
    transaction::{InputNote, TransactionId},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    BlockHeader, Digest,
};

use super::{NoteRecordError, NoteState, NoteSubmissionData};

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

    // PUBLIC ACCESSORS
    // ================================================================================================

    pub fn id(&self) -> NoteId {
        self.details.id()
    }

    pub fn recipient(&self) -> Digest {
        self.details.recipient().digest()
    }

    pub fn assets(&self) -> &NoteAssets {
        self.details.assets()
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
        matches!(
            self.state,
            NoteState::Committed { .. }
                | NoteState::ProcessingAuthenticated { .. }
                | NoteState::NativeConsumed { .. }
        )
    }

    // TRANSITIONS
    // ================================================================================================

    pub fn inclusion_proof_received(
        &mut self,
        inclusion_proof: NoteInclusionProof,
        metadata: NoteMetadata,
    ) -> Result<bool, NoteRecordError> {
        match &self.state {
            // Note had no inclusion proof
            NoteState::Expected { .. } | NoteState::Unknown => {
                self.state = NoteState::Unverified { inclusion_proof, metadata };
                Ok(true)
            },
            // Note had an inclusion proof
            NoteState::Unverified {
                metadata: old_metadata,
                inclusion_proof: old_inclusion_proof,
            }
            | NoteState::Committed {
                metadata: old_metadata,
                inclusion_proof: old_inclusion_proof,
                ..
            }
            | NoteState::ProcessingAuthenticated {
                metadata: old_metadata,
                inclusion_proof: old_inclusion_proof,
                ..
            } => {
                if old_inclusion_proof != &inclusion_proof || old_metadata != &metadata {
                    return Err(NoteRecordError::StateTransitionError(
                        "Inclusion proof or metadata do not match the expected values".to_string(),
                    ));
                }
                Ok(false)
            },
            _ => todo!("How should we deal with invalid/unnecessary transitions? Maybe we don't want a default '_' case"),
        }
    }

    pub fn block_header_received(
        &mut self,
        block_header: BlockHeader,
    ) -> Result<bool, NoteRecordError> {
        match &self.state {
            NoteState::Unverified { inclusion_proof, metadata } => {
                if inclusion_proof.location().block_num() != block_header.block_num() {
                    return Err(NoteRecordError::StateTransitionError(
                        "Block header does not match the block number in the inclusion proof"
                            .to_string(),
                    ));
                }

                self.state = if inclusion_proof.note_path().verify(
                    inclusion_proof.location().node_index_in_block().into(),
                    compute_note_hash(self.id(), metadata),
                    &block_header.note_root(),
                ) {
                    NoteState::Committed {
                        inclusion_proof: inclusion_proof.clone(),
                        metadata: *metadata,
                        block_note_root: block_header.note_root(),
                    }
                } else {
                    NoteState::Invalid {
                        invalid_inclusion_proof: inclusion_proof.clone(),
                        metadata: *metadata,
                        block_note_root: block_header.note_root(),
                    }
                };
                Ok(true)
            },
            _ => todo!("How should we deal with invalid/unnecessary transitions? Maybe we don't want a default '_' case"),
       }
    }

    pub fn nullifier_received(
        &mut self,
        nullifier: Nullifier,
        nullifier_block_height: u32,
    ) -> Result<bool, NoteRecordError> {
        match &self.state {
            NoteState::ProcessingAuthenticated { metadata, inclusion_proof, block_note_root, submission_data } => {
                if self.nullifier() != nullifier {
                    return Err(NoteRecordError::StateTransitionError(
                        "Nullifier does not match the expected value".to_string(),
                    ));
                }
                self.state = NoteState::NativeConsumed { metadata: *metadata, inclusion_proof: inclusion_proof.clone(), block_note_root: *block_note_root, nullifier_block_height , submission_data: *submission_data };
                Ok(true)
            },
            NoteState::ProcessingUnauthenticated { after_block_num: _, submission_data: _ } => {
                todo!("This should be NativeConsumed, but we don't have the metadata to set it.")
            }
            NoteState::Unknown | NoteState::Expected { .. } | NoteState::Committed { .. } | NoteState::Invalid { .. } | NoteState::Unverified { .. }  => {
                self.state = NoteState::ForeignConsumed { nullifier_block_height };
                Ok(true)
            },
            _ => todo!("How should we deal with invalid/unnecessary transitions? Maybe we don't want a default '_' case"),
        }
    }

    pub fn consumed_locally(
        &mut self,
        consumer_account: AccountId,
        consumer_transaction: TransactionId,
    ) -> Result<bool, NoteRecordError> {
        match &self.state {
            NoteState::Committed { metadata, inclusion_proof, block_note_root } => {
                let submission_data = NoteSubmissionData{
                    submitted_at: None,
                    consumer_account,
                    consumer_transaction,
                };

                self.state = NoteState::ProcessingAuthenticated { metadata: *metadata, inclusion_proof: inclusion_proof.clone(), block_note_root: *block_note_root, submission_data };
                Ok(true)
            },
            _ => todo!("How should we deal with invalid/unnecessary transitions? Maybe we don't want a default '_' case"),
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
        let tag = value.metadata().tag();
        Self {
            details: value.into(),
            created_at: None,
            state: NoteState::Expected { after_block_num: 0, tag },
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
                    metadata: *note.metadata(),
                    inclusion_proof: proof,
                },
            },
            InputNote::Unauthenticated { note } => note.into(),
        }
    }
}

impl TryInto<InputNote> for InputNoteRecord {
    type Error = NoteRecordError;

    fn try_into(self) -> Result<InputNote, Self::Error> {
        match (self.metadata(), self.inclusion_proof()) {
            (Some(metadata), Some(inclusion_proof)) => Ok(InputNote::authenticated(
                Note::new(
                    self.details.assets().clone(),
                    *metadata,
                    self.details.recipient().clone(),
                ),
                inclusion_proof.clone(),
            )),
            (Some(metadata), None) => Ok(InputNote::unauthenticated(Note::new(
                self.details.assets().clone(),
                *metadata,
                self.details.recipient().clone(),
            ))),
            _ => Err(NoteRecordError::ConversionError(
                "Input Note Record contains no metadata".to_string(),
            )),
        }
    }
}

impl TryInto<Note> for InputNoteRecord {
    type Error = NoteRecordError;

    fn try_into(self) -> Result<Note, Self::Error> {
        match self.metadata().cloned() {
            Some(metadata) => Ok(Note::new(
                self.details.assets().clone(),
                metadata,
                self.details.recipient().clone(),
            )),
            None => Err(NoteRecordError::ConversionError(
                "Input Note Record contains no metadata".to_string(),
            )),
        }
    }
}

impl From<InputNoteRecord> for NoteDetails {
    fn from(value: InputNoteRecord) -> Self {
        value.details
    }
}

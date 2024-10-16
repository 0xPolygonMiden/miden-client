use alloc::string::ToString;

use miden_objects::{
    accounts::AccountId,
    notes::{Note, NoteAssets, NoteDetails, NoteId, NoteInclusionProof, NoteMetadata, Nullifier},
    transaction::{InputNote, TransactionId},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    BlockHeader, Digest,
};

use super::NoteRecordError;

mod states;
pub use states::{
    CommittedNoteState, ConsumedAuthenticatedLocalNoteState, ExpectedNoteState, InputNoteState,
    InvalidNoteState, ProcessingAuthenticatedNoteState, ProcessingUnauthenticatedNoteState,
    UnverifiedNoteState,
};

// INPUT NOTE RECORD
// ================================================================================================

/// Represents a Note of which the Store can keep track and retrieve.
///
/// An [InputNoteRecord] contains all the information of a [NoteDetails], in addition of specific
/// information about the note state.
///
/// Once a proof is received, the [InputNoteRecord] can be transformed into an [InputNote] and used
/// as input for transactions.
/// It is also possible to convert [Note] and [InputNote] into [InputNoteRecord] (we fill the
/// `metadata` and `inclusion_proof` fields if possible)
#[derive(Clone, Debug, PartialEq)]
pub struct InputNoteRecord {
    /// Details of a note consisting of assets, script, inputs, and a serial number.
    details: NoteDetails,
    /// The timestamp at which the note was created. If it's not known, it will be None.
    created_at: Option<u64>,
    /// The state of the note, with specific fields for each one.
    state: InputNoteState,
}

impl InputNoteRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        details: NoteDetails,
        created_at: Option<u64>,
        state: InputNoteState,
    ) -> InputNoteRecord {
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

    pub fn state(&self) -> &InputNoteState {
        &self.state
    }

    pub fn metadata(&self) -> Option<&NoteMetadata> {
        self.state.metadata()
    }

    pub fn nullifier(&self) -> Nullifier {
        self.details.nullifier()
    }

    pub fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        self.state.inclusion_proof()
    }

    pub fn details(&self) -> &NoteDetails {
        &self.details
    }

    pub fn consumer_transaction_id(&self) -> Option<&TransactionId> {
        self.state.consumer_transaction_id()
    }

    pub fn is_authenticated(&self) -> bool {
        matches!(
            self.state,
            InputNoteState::Committed { .. }
                | InputNoteState::ProcessingAuthenticated { .. }
                | InputNoteState::ConsumedAuthenticatedLocal { .. }
        )
    }

    pub fn is_consumed(&self) -> bool {
        matches!(
            self.state,
            InputNoteState::ConsumedExternal { .. }
                | InputNoteState::ConsumedAuthenticatedLocal { .. }
                | InputNoteState::ConsumedUnauthenticatedLocal { .. }
        )
    }

    pub fn is_processing(&self) -> bool {
        matches!(
            self.state,
            InputNoteState::ProcessingAuthenticated { .. }
                | InputNoteState::ProcessingUnauthenticated { .. }
        )
    }

    // TRANSITIONS
    // ================================================================================================

    /// Modifies the state of the note record to reflect that the it has received an inclusion
    /// proof. It is assumed to be unverified until the block header information is received.
    /// Returns `true` if the state was changed.
    pub fn inclusion_proof_received(
        &mut self,
        inclusion_proof: NoteInclusionProof,
        metadata: NoteMetadata,
    ) -> Result<bool, NoteRecordError> {
        let new_state = self.state.inclusion_proof_received(inclusion_proof, metadata)?;
        if let Some(new_state) = new_state {
            self.state = new_state;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Modifies the state of the note record to reflect that the it has received a block header.
    /// This will mark the note as verified or invalid, depending on the block header
    /// information and inclusion proof. Returns `true` if the state was changed.
    pub fn block_header_received(
        &mut self,
        block_header: BlockHeader,
    ) -> Result<bool, NoteRecordError> {
        let new_state = self.state.block_header_received(self.id(), block_header)?;
        if let Some(new_state) = new_state {
            self.state = new_state;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Modifies the state of the note record to reflect that the note has been consumed by an
    /// external transaction. Returns `true` if the state was changed.
    ///
    /// Errors:
    /// - If the nullifier does not match the expected value.
    pub fn consumed_externally(
        &mut self,
        nullifier: Nullifier,
        nullifier_block_height: u32,
    ) -> Result<bool, NoteRecordError> {
        if self.nullifier() != nullifier {
            return Err(NoteRecordError::StateTransitionError(
                "Nullifier does not match the expected value".to_string(),
            ));
        }

        let new_state = self.state.consumed_externally(nullifier_block_height)?;
        if let Some(new_state) = new_state {
            self.state = new_state;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Modifies the state of the note record to reflect that the client began processing the note
    /// to be consumed. Returns `true` if the state was changed.
    pub fn consumed_locally(
        &mut self,
        consumer_account: AccountId,
        consumer_transaction: TransactionId,
    ) -> Result<bool, NoteRecordError> {
        let new_state = self.state.consumed_locally(consumer_account, consumer_transaction)?;
        if let Some(new_state) = new_state {
            self.state = new_state;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Modifies the state of the note record to reflect that the transaction currently consuming
    /// the note was committed. Returns `true` if the state was changed.3
    pub fn transaction_committed(
        &mut self,
        transaction_id: TransactionId,
        block_height: u32,
    ) -> Result<bool, NoteRecordError> {
        let new_state = self.state.transaction_committed(transaction_id, block_height)?;
        if let Some(new_state) = new_state {
            self.state = new_state;
            Ok(true)
        } else {
            Ok(false)
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
        let state = InputNoteState::read_from(source)?;

        Ok(InputNoteRecord { details, created_at, state })
    }
}

// CONVERSION
// ================================================================================================
impl From<Note> for InputNoteRecord {
    fn from(value: Note) -> Self {
        let metadata = *value.metadata();
        Self {
            details: value.into(),
            created_at: None,
            state: ExpectedNoteState {
                metadata: Some(metadata),
                after_block_num: 0,
                tag: Some(metadata.tag()),
            }
            .into(),
        }
    }
}

impl From<InputNote> for InputNoteRecord {
    fn from(value: InputNote) -> Self {
        match value {
            InputNote::Authenticated { note, proof } => Self {
                details: note.clone().into(),
                created_at: None,
                state: UnverifiedNoteState {
                    metadata: *note.metadata(),
                    inclusion_proof: proof,
                }
                .into(),
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

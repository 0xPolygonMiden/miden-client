use alloc::string::ToString;
use core::fmt::{self, Display};

use chrono::{Local, TimeZone};
use miden_objects::{
    accounts::AccountId,
    notes::{NoteId, NoteInclusionProof, NoteMetadata},
    transaction::TransactionId,
    BlockHeader,
};

mod committed;
mod consumed_authenticated_local;
mod consumed_external;
mod consumed_unauthenticated_local;
mod expected;
mod invalid;
mod processing_authenticated;
mod processing_unauthenticated;
mod unverified;

pub use committed::CommittedNoteState;
pub use consumed_authenticated_local::ConsumedAuthenticatedLocalNoteState;
pub use consumed_external::ConsumedExternalNoteState;
pub use consumed_unauthenticated_local::ConsumedUnauthenticatedLocalNoteState;
pub use expected::ExpectedNoteState;
pub use invalid::InvalidNoteState;
pub use miden_tx::utils::{
    ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
};
pub use processing_authenticated::ProcessingAuthenticatedNoteState;
pub use processing_unauthenticated::ProcessingUnauthenticatedNoteState;
pub use unverified::UnverifiedNoteState;

use super::NoteRecordError;

pub const STATE_EXPECTED: u8 = 0;
pub const STATE_UNVERIFIED: u8 = 1;
pub const STATE_COMMITTED: u8 = 2;
pub const STATE_INVALID: u8 = 3;
pub const STATE_PROCESSING_AUTHENTICATED: u8 = 4;
pub const STATE_PROCESSING_UNAUTHENTICATED: u8 = 5;
pub const STATE_CONSUMED_AUTHENTICATED_LOCAL: u8 = 6;
pub const STATE_CONSUMED_UNAUTHENTICATED_LOCAL: u8 = 7;
pub const STATE_CONSUMED_EXTERNAL: u8 = 8;

#[derive(Clone, Debug, PartialEq)]
pub enum NoteState {
    /// Tracked by the client but without a chain inclusion proof.
    Expected(ExpectedNoteState),
    /// With inclusion proof but not yet verified.
    Unverified(UnverifiedNoteState),
    /// With verified inclusion proof.
    Committed(CommittedNoteState),
    /// With invalid inclusion proof.
    Invalid(InvalidNoteState),
    /// Authenticated note being consumed locally by the client, awaiting chain confirmation.
    ProcessingAuthenticated(ProcessingAuthenticatedNoteState),
    /// Unauthenticated note being consumed locally by the client, awaiting chain confirmation.
    ProcessingUnauthenticated(ProcessingUnauthenticatedNoteState),
    /// Authenticated note consumed locally by the client and confirmed by the chain.
    ConsumedAuthenticatedLocal(ConsumedAuthenticatedLocalNoteState),
    /// Unauthenticated note consumed locally by the client and confirmed by the chain.
    ConsumedUnauthenticatedLocal(ConsumedUnauthenticatedLocalNoteState),
    /// Note consumed in chain by an external account (e.g. an account not tracked by the client).
    ConsumedExternal(ConsumedExternalNoteState),
}

impl NoteState {
    /// Returns the inner state handler that implements state transitions.
    fn inner(&self) -> &dyn NoteStateHandler {
        match self {
            NoteState::Expected(inner) => inner,
            NoteState::Unverified(inner) => inner,
            NoteState::Committed(inner) => inner,
            NoteState::Invalid(inner) => inner,
            NoteState::ProcessingAuthenticated(inner) => inner,
            NoteState::ProcessingUnauthenticated(inner) => inner,
            NoteState::ConsumedAuthenticatedLocal(inner) => inner,
            NoteState::ConsumedUnauthenticatedLocal(inner) => inner,
            NoteState::ConsumedExternal(inner) => inner,
        }
    }

    pub fn metadata(&self) -> Option<&NoteMetadata> {
        self.inner().metadata()
    }

    pub fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        self.inner().inclusion_proof()
    }

    pub fn consumer_transaction_id(&self) -> Option<&TransactionId> {
        self.inner().consumer_transaction_id()
    }

    /// Returns a unique identifier for each note state.
    pub fn discriminant(&self) -> u8 {
        match self {
            NoteState::Expected(_) => STATE_EXPECTED,
            NoteState::Unverified(_) => STATE_UNVERIFIED,
            NoteState::Committed(_) => STATE_COMMITTED,
            NoteState::Invalid(_) => STATE_INVALID,
            NoteState::ProcessingAuthenticated(_) => STATE_PROCESSING_AUTHENTICATED,
            NoteState::ProcessingUnauthenticated(_) => STATE_PROCESSING_UNAUTHENTICATED,
            NoteState::ConsumedAuthenticatedLocal(_) => STATE_CONSUMED_AUTHENTICATED_LOCAL,
            NoteState::ConsumedUnauthenticatedLocal(_) => STATE_CONSUMED_UNAUTHENTICATED_LOCAL,
            NoteState::ConsumedExternal(_) => STATE_CONSUMED_EXTERNAL,
        }
    }

    /// Returns a new state to reflect that the note has received an inclusion proof. The proof is
    /// assumed to be unverified until the block header information is received. If the note state
    /// doesn't change, `None` is returned.
    pub fn inclusion_proof_received(
        &self,
        inclusion_proof: NoteInclusionProof,
        metadata: NoteMetadata,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        self.inner().inclusion_proof_received(inclusion_proof, metadata)
    }

    /// Returns a new to reflect that its nullifier has been received, meaning that the note has
    /// been spent. If the note state doesn't change, `None` is returned.
    ///
    /// Errors:
    /// - If the nullifier does not match the expected value.
    pub fn consumed_externally(
        &self,
        nullifier_block_height: u32,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        self.inner().consumed_externally(nullifier_block_height)
    }

    /// Returns a new state to reflect that the note has received a block header.
    /// This will mark the note as verified or invalid, depending on the block header
    /// information and inclusion proof. If the note state
    /// doesn't change, `None` is returned.
    pub fn block_header_received(
        &self,
        note_id: NoteId,
        block_header: BlockHeader,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        self.inner().block_header_received(note_id, block_header)
    }

    /// Modifies the state of the note record to reflect that the client began processing the note
    /// to be consumed. If the note state doesn't change, `None` is returned.
    pub fn consumed_locally(
        &self,
        consumer_account: AccountId,
        consumer_transaction: TransactionId,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        self.inner().consumed_locally(consumer_account, consumer_transaction)
    }

    pub fn transaction_committed(
        &self,
        transaction_id: TransactionId,
        block_height: u32,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        self.inner().transaction_committed(transaction_id, block_height)
    }
}

impl Serializable for NoteState {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_u8(self.discriminant());
        match self {
            NoteState::Expected(inner) => inner.write_into(target),
            NoteState::Unverified(inner) => inner.write_into(target),
            NoteState::Committed(inner) => inner.write_into(target),
            NoteState::Invalid(inner) => inner.write_into(target),
            NoteState::ProcessingAuthenticated(inner) => inner.write_into(target),
            NoteState::ProcessingUnauthenticated(inner) => inner.write_into(target),
            NoteState::ConsumedAuthenticatedLocal(inner) => inner.write_into(target),
            NoteState::ConsumedUnauthenticatedLocal(inner) => inner.write_into(target),
            NoteState::ConsumedExternal(inner) => inner.write_into(target),
        }
    }
}

impl Deserializable for NoteState {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let discriminant = source.read_u8()?;
        match discriminant {
            STATE_EXPECTED => Ok(ExpectedNoteState::read_from(source)?.into()),
            STATE_UNVERIFIED => Ok(UnverifiedNoteState::read_from(source)?.into()),
            STATE_COMMITTED => Ok(CommittedNoteState::read_from(source)?.into()),
            STATE_INVALID => Ok(InvalidNoteState::read_from(source)?.into()),
            STATE_PROCESSING_AUTHENTICATED => {
                Ok(ProcessingAuthenticatedNoteState::read_from(source)?.into())
            },
            STATE_PROCESSING_UNAUTHENTICATED => {
                Ok(ProcessingUnauthenticatedNoteState::read_from(source)?.into())
            },
            STATE_CONSUMED_AUTHENTICATED_LOCAL => {
                Ok(ConsumedAuthenticatedLocalNoteState::read_from(source)?.into())
            },
            STATE_CONSUMED_UNAUTHENTICATED_LOCAL => {
                Ok(ConsumedUnauthenticatedLocalNoteState::read_from(source)?.into())
            },
            STATE_CONSUMED_EXTERNAL => Ok(ConsumedExternalNoteState::read_from(source)?.into()),
            _ => Err(DeserializationError::InvalidValue(format!(
                "Invalid NoteState discriminant: {}",
                discriminant
            ))),
        }
    }
}

impl Display for NoteState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteState::Expected(state) => {
                write!(f, "Expected (after block {})", state.after_block_num)
            },
            NoteState::Unverified(state) => {
                write!(
                    f,
                    "Unverified (with commit block {})",
                    state.inclusion_proof.location().block_num()
                )
            },
            NoteState::Committed(state) => {
                write!(
                    f,
                    "Committed (at block height {})",
                    state.inclusion_proof.location().block_num()
                )
            },
            NoteState::Invalid(state) => {
                write!(
                    f,
                    "Invalid (with commit block {})",
                    state.invalid_inclusion_proof.location().block_num()
                )
            },
            NoteState::ProcessingAuthenticated(ProcessingAuthenticatedNoteState {
                submission_data,
                ..
            })
            | NoteState::ProcessingUnauthenticated(ProcessingUnauthenticatedNoteState {
                submission_data,
                ..
            }) => {
                write!(
                    f,
                    "Processing (submitted at {} by account {})",
                    submission_data
                        .submitted_at
                        .map(|submitted_at| {
                            Local
                                .timestamp_opt(submitted_at as i64, 0)
                                .single()
                                .expect("timestamp should be valid")
                                .to_string()
                        })
                        .unwrap_or("?".to_string()),
                    submission_data.consumer_account
                )
            },
            NoteState::ConsumedAuthenticatedLocal(ConsumedAuthenticatedLocalNoteState {
                nullifier_block_height,
                submission_data,
                ..
            })
            | NoteState::ConsumedUnauthenticatedLocal(ConsumedUnauthenticatedLocalNoteState {
                nullifier_block_height,
                submission_data,
                ..
            }) => {
                write!(
                    f,
                    "Consumed (at block {} by account {})",
                    nullifier_block_height, submission_data.consumer_account
                )
            },
            NoteState::ConsumedExternal(state) => {
                write!(f, "Consumed (at block {})", state.nullifier_block_height)
            },
        }
    }
}

pub trait NoteStateHandler {
    fn metadata(&self) -> Option<&NoteMetadata>;

    fn inclusion_proof(&self) -> Option<&NoteInclusionProof>;

    fn consumer_transaction_id(&self) -> Option<&TransactionId>;

    fn inclusion_proof_received(
        &self,
        inclusion_proof: NoteInclusionProof,
        metadata: NoteMetadata,
    ) -> Result<Option<NoteState>, NoteRecordError>;

    fn consumed_externally(
        &self,
        nullifier_block_height: u32,
    ) -> Result<Option<NoteState>, NoteRecordError>;

    fn block_header_received(
        &self,
        note_id: NoteId,
        block_header: BlockHeader,
    ) -> Result<Option<NoteState>, NoteRecordError>;

    fn consumed_locally(
        &self,
        consumer_account: AccountId,
        consumer_transaction: TransactionId,
    ) -> Result<Option<NoteState>, NoteRecordError>;

    fn transaction_committed(
        &self,
        transaction_id: TransactionId,
        block_height: u32,
    ) -> Result<Option<NoteState>, NoteRecordError>;
}

/// Information about a locally consumed note submitted to the node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoteSubmissionData {
    /// The timestamp at which the note was submitted.
    pub submitted_at: Option<u64>,
    /// The ID of the account that is consuming the note.
    pub consumer_account: AccountId,
    /// The ID of the transaction that is consuming the note.
    pub consumer_transaction: TransactionId,
}

impl Serializable for NoteSubmissionData {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.submitted_at.write_into(target);
        self.consumer_account.write_into(target);
        self.consumer_transaction.write_into(target);
    }
}

impl Deserializable for NoteSubmissionData {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let submitted_at = Option::<u64>::read_from(source)?;
        let consumer_account = AccountId::read_from(source)?;
        let consumer_transaction = TransactionId::read_from(source)?;
        Ok(NoteSubmissionData {
            submitted_at,
            consumer_account,
            consumer_transaction,
        })
    }
}

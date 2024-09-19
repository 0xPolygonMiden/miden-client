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
    Expected(ExpectedNoteState),
    Unverified(UnverifiedNoteState),
    Committed(CommittedNoteState),
    Invalid(InvalidNoteState),
    ProcessingAuthenticated(ProcessingAuthenticatedNoteState),
    ProcessingUnauthenticated(ProcessingUnauthenticatedNoteState),
    ConsumedAuthenticatedLocal(ConsumedAuthenticatedLocalNoteState),
    ConsumedUnauthenticatedLocal(ConsumedUnauthenticatedLocalNoteState),
    ConsumedExternal(ConsumedExternalNoteState),
}

impl NoteState {
    pub fn inner(&self) -> &dyn NoteStateHandler {
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
            STATE_EXPECTED => {
                let state = ExpectedNoteState::read_from(source)?;
                Ok(NoteState::Expected(state))
            },
            STATE_UNVERIFIED => {
                let state = UnverifiedNoteState::read_from(source)?;
                Ok(NoteState::Unverified(state))
            },
            STATE_COMMITTED => {
                let state = CommittedNoteState::read_from(source)?;
                Ok(NoteState::Committed(state))
            },
            STATE_INVALID => {
                let state = InvalidNoteState::read_from(source)?;
                Ok(NoteState::Invalid(state))
            },
            STATE_PROCESSING_AUTHENTICATED => {
                let state = ProcessingAuthenticatedNoteState::read_from(source)?;
                Ok(NoteState::ProcessingAuthenticated(state))
            },
            STATE_PROCESSING_UNAUTHENTICATED => {
                let state = ProcessingUnauthenticatedNoteState::read_from(source)?;
                Ok(NoteState::ProcessingUnauthenticated(state))
            },
            STATE_CONSUMED_AUTHENTICATED_LOCAL => {
                let state = ConsumedAuthenticatedLocalNoteState::read_from(source)?;
                Ok(NoteState::ConsumedAuthenticatedLocal(state))
            },
            STATE_CONSUMED_UNAUTHENTICATED_LOCAL => {
                let state = ConsumedUnauthenticatedLocalNoteState::read_from(source)?;
                Ok(NoteState::ConsumedUnauthenticatedLocal(state))
            },
            STATE_CONSUMED_EXTERNAL => {
                let state = ConsumedExternalNoteState::read_from(source)?;
                Ok(NoteState::ConsumedExternal(state))
            },
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

    fn inclusion_proof_received(
        &self,
        inclusion_proof: NoteInclusionProof,
        metadata: NoteMetadata,
    ) -> Result<Option<NoteState>, NoteRecordError>;

    fn nullifier_received(
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoteSubmissionData {
    pub submitted_at: Option<u64>,
    pub consumer_account: AccountId,
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

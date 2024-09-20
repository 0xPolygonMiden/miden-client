use alloc::string::ToString;

use miden_objects::{
    notes::{NoteId, NoteInclusionProof, NoteMetadata},
    BlockHeader,
};

use super::{
    ConsumedUnauthenticatedLocalNoteState, NoteState, NoteStateHandler, NoteSubmissionData,
    STATE_PROCESSING_UNAUTHENTICATED,
};
use crate::store::NoteRecordError;

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessingUnauthenticatedNoteState {
    pub metadata: NoteMetadata,
    pub after_block_num: u32,
    pub submission_data: NoteSubmissionData,
}

impl NoteStateHandler for ProcessingUnauthenticatedNoteState {
    fn inclusion_proof_received(
        &self,
        _inclusion_proof: NoteInclusionProof,
        _metadata: NoteMetadata,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        Err(NoteRecordError::InvalidStateTransition {
            state: STATE_PROCESSING_UNAUTHENTICATED,
            transition_name: "inclusion_proof_received".to_string(),
        })
    }

    fn nullifier_received(
        &self,
        nullifier_block_height: u32,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        Ok(Some(NoteState::ConsumedUnauthenticatedLocal(
            ConsumedUnauthenticatedLocalNoteState {
                metadata: self.metadata,
                nullifier_block_height,
                submission_data: self.submission_data,
            },
        )))
    }

    fn block_header_received(
        &self,
        _note_id: NoteId,
        _block_header: BlockHeader,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        Ok(None)
    }

    fn consumed_locally(
        &self,
        _consumer_account: miden_objects::accounts::AccountId,
        _consumer_transaction: miden_objects::transaction::TransactionId,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        Err(NoteRecordError::InvalidStateTransition {
            state: STATE_PROCESSING_UNAUTHENTICATED,
            transition_name: "consumed_locally".to_string(),
        })
    }

    fn metadata(&self) -> Option<&NoteMetadata> {
        Some(&self.metadata)
    }

    fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        None
    }
}

impl miden_tx::utils::Serializable for ProcessingUnauthenticatedNoteState {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        self.metadata.write_into(target);
        self.after_block_num.write_into(target);
        self.submission_data.write_into(target);
    }
}

impl miden_tx::utils::Deserializable for ProcessingUnauthenticatedNoteState {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_tx::utils::DeserializationError> {
        let metadata = NoteMetadata::read_from(source)?;
        let after_block_num = u32::read_from(source)?;
        let submission_data = NoteSubmissionData::read_from(source)?;
        Ok(ProcessingUnauthenticatedNoteState {
            metadata,
            after_block_num,
            submission_data,
        })
    }
}

impl From<ProcessingUnauthenticatedNoteState> for NoteState {
    fn from(state: ProcessingUnauthenticatedNoteState) -> Self {
        NoteState::ProcessingUnauthenticated(state)
    }
}

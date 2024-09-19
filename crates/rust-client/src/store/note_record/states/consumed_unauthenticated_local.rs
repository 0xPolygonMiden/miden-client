use alloc::string::ToString;

use miden_objects::{
    notes::{NoteId, NoteInclusionProof, NoteMetadata},
    BlockHeader,
};

use super::{
    NoteState, NoteStateHandler, NoteSubmissionData, STATE_CONSUMED_UNAUTHENTICATED_LOCAL,
};
use crate::store::NoteRecordError;

#[derive(Clone, Debug, PartialEq)]
pub struct ConsumedUnauthenticatedLocalNoteState {
    pub metadata: NoteMetadata,
    pub nullifier_block_height: u32,
    pub submission_data: NoteSubmissionData,
}

impl NoteStateHandler for ConsumedUnauthenticatedLocalNoteState {
    fn inclusion_proof_received(
        &self,
        _inclusion_proof: NoteInclusionProof,
        _metadata: NoteMetadata,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        Ok(None)
    }

    fn nullifier_received(
        &self,
        _nullifier_block_height: u32,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        Ok(None)
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
            state: STATE_CONSUMED_UNAUTHENTICATED_LOCAL,
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

impl miden_tx::utils::Serializable for ConsumedUnauthenticatedLocalNoteState {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        self.metadata.write_into(target);
        self.nullifier_block_height.write_into(target);
        self.submission_data.write_into(target);
    }
}

impl miden_tx::utils::Deserializable for ConsumedUnauthenticatedLocalNoteState {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_tx::utils::DeserializationError> {
        let metadata = NoteMetadata::read_from(source)?;
        let nullifier_block_height = u32::read_from(source)?;
        let submission_data = NoteSubmissionData::read_from(source)?;
        Ok(ConsumedUnauthenticatedLocalNoteState {
            metadata,
            nullifier_block_height,
            submission_data,
        })
    }
}

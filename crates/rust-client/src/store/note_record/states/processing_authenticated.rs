use alloc::string::ToString;

use miden_objects::{
    accounts::AccountId,
    notes::{NoteId, NoteInclusionProof, NoteMetadata},
    transaction::TransactionId,
    BlockHeader, Digest,
};

use super::{
    ConsumedAuthenticatedLocalNoteState, NoteState, NoteStateHandler, NoteSubmissionData,
    STATE_PROCESSING_AUTHENTICATED,
};
use crate::store::NoteRecordError;

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessingAuthenticatedNoteState {
    pub metadata: NoteMetadata,
    pub inclusion_proof: NoteInclusionProof,
    pub block_note_root: Digest,
    pub submission_data: NoteSubmissionData,
}

impl NoteStateHandler for ProcessingAuthenticatedNoteState {
    fn inclusion_proof_received(
        &self,
        inclusion_proof: NoteInclusionProof,
        metadata: NoteMetadata,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        if self.inclusion_proof != inclusion_proof || self.metadata != metadata {
            return Err(NoteRecordError::StateTransitionError(
                "Inclusion proof or metadata do not match the expected values".to_string(),
            ));
        }
        Ok(None)
    }

    fn nullifier_received(
        &self,
        nullifier_block_height: u32,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        Ok(Some(
            ConsumedAuthenticatedLocalNoteState {
                metadata: self.metadata,
                inclusion_proof: self.inclusion_proof.clone(),
                block_note_root: self.block_note_root,
                nullifier_block_height,
                submission_data: self.submission_data,
            }
            .into(),
        ))
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
        _consumer_account: AccountId,
        _consumer_transaction: TransactionId,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        Err(NoteRecordError::InvalidStateTransition {
            state: STATE_PROCESSING_AUTHENTICATED,
            transition_name: "consumed_locally".to_string(),
        })
    }

    fn metadata(&self) -> Option<&NoteMetadata> {
        Some(&self.metadata)
    }

    fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        Some(&self.inclusion_proof)
    }
}

impl miden_tx::utils::Serializable for ProcessingAuthenticatedNoteState {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        self.metadata.write_into(target);
        self.inclusion_proof.write_into(target);
        self.block_note_root.write_into(target);
        self.submission_data.write_into(target);
    }
}

impl miden_tx::utils::Deserializable for ProcessingAuthenticatedNoteState {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_tx::utils::DeserializationError> {
        let metadata = NoteMetadata::read_from(source)?;
        let inclusion_proof = NoteInclusionProof::read_from(source)?;
        let block_note_root = Digest::read_from(source)?;
        let submission_data = NoteSubmissionData::read_from(source)?;
        Ok(ProcessingAuthenticatedNoteState {
            metadata,
            inclusion_proof,
            block_note_root,
            submission_data,
        })
    }
}

impl From<ProcessingAuthenticatedNoteState> for NoteState {
    fn from(state: ProcessingAuthenticatedNoteState) -> Self {
        NoteState::ProcessingAuthenticated(state)
    }
}

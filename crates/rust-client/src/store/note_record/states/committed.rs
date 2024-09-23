use alloc::string::ToString;

use miden_objects::{
    notes::{NoteId, NoteInclusionProof, NoteMetadata},
    BlockHeader, Digest,
};

use super::{
    ConsumedExternalNoteState, NoteState, NoteStateHandler, NoteSubmissionData,
    ProcessingAuthenticatedNoteState,
};
use crate::store::NoteRecordError;

#[derive(Clone, Debug, PartialEq)]
pub struct CommittedNoteState {
    pub metadata: NoteMetadata,
    pub inclusion_proof: NoteInclusionProof,
    pub block_note_root: Digest,
}

impl NoteStateHandler for CommittedNoteState {
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
        Ok(Some(ConsumedExternalNoteState { nullifier_block_height }.into()))
    }

    fn block_header_received(
        &self,
        _note_id: NoteId,
        block_header: BlockHeader,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        if block_header.note_root() != self.block_note_root {
            return Err(NoteRecordError::StateTransitionError(
                "Block header does not match the expected note root".to_string(),
            ));
        }
        Ok(None)
    }

    fn consumed_locally(
        &self,
        consumer_account: miden_objects::accounts::AccountId,
        consumer_transaction: miden_objects::transaction::TransactionId,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        let submission_data = NoteSubmissionData {
            submitted_at: None,
            consumer_account,
            consumer_transaction,
        };

        Ok(Some(
            ProcessingAuthenticatedNoteState {
                metadata: self.metadata,
                inclusion_proof: self.inclusion_proof.clone(),
                block_note_root: self.block_note_root,
                submission_data,
            }
            .into(),
        ))
    }

    fn metadata(&self) -> Option<&NoteMetadata> {
        Some(&self.metadata)
    }

    fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        Some(&self.inclusion_proof)
    }
}

impl miden_tx::utils::Serializable for CommittedNoteState {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        self.metadata.write_into(target);
        self.inclusion_proof.write_into(target);
        self.block_note_root.write_into(target);
    }
}

impl miden_tx::utils::Deserializable for CommittedNoteState {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_tx::utils::DeserializationError> {
        let metadata = NoteMetadata::read_from(source)?;
        let inclusion_proof = NoteInclusionProof::read_from(source)?;
        let block_note_root = Digest::read_from(source)?;
        Ok(CommittedNoteState {
            metadata,
            inclusion_proof,
            block_note_root,
        })
    }
}

impl From<CommittedNoteState> for NoteState {
    fn from(state: CommittedNoteState) -> Self {
        NoteState::Committed(state)
    }
}

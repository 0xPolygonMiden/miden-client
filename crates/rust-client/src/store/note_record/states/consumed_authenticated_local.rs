use alloc::string::ToString;

use miden_objects::{
    notes::{NoteId, NoteInclusionProof, NoteMetadata},
    BlockHeader, Digest,
};

use super::{NoteState, NoteStateHandler, NoteSubmissionData};
use crate::store::NoteRecordError;

#[derive(Clone, Debug, PartialEq)]
pub struct ConsumedAuthenticatedLocalNoteState {
    pub metadata: NoteMetadata,
    pub inclusion_proof: NoteInclusionProof,
    pub block_note_root: Digest,
    pub nullifier_block_height: u32,
    pub submission_data: NoteSubmissionData,
}

impl NoteStateHandler for ConsumedAuthenticatedLocalNoteState {
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
        Err(NoteRecordError::NoteNotConsumable("Note already consumed".to_string()))
    }

    fn metadata(&self) -> Option<&NoteMetadata> {
        Some(&self.metadata)
    }

    fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        Some(&self.inclusion_proof)
    }
}

impl miden_tx::utils::Serializable for ConsumedAuthenticatedLocalNoteState {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        self.metadata.write_into(target);
        self.inclusion_proof.write_into(target);
        self.block_note_root.write_into(target);
        self.nullifier_block_height.write_into(target);
        self.submission_data.write_into(target);
    }
}

impl miden_tx::utils::Deserializable for ConsumedAuthenticatedLocalNoteState {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_tx::utils::DeserializationError> {
        let metadata = NoteMetadata::read_from(source)?;
        let inclusion_proof = NoteInclusionProof::read_from(source)?;
        let block_note_root = Digest::read_from(source)?;
        let nullifier_block_height = u32::read_from(source)?;
        let submission_data = NoteSubmissionData::read_from(source)?;
        Ok(ConsumedAuthenticatedLocalNoteState {
            metadata,
            inclusion_proof,
            block_note_root,
            nullifier_block_height,
            submission_data,
        })
    }
}

impl From<ConsumedAuthenticatedLocalNoteState> for NoteState {
    fn from(state: ConsumedAuthenticatedLocalNoteState) -> Self {
        NoteState::ConsumedAuthenticatedLocal(state)
    }
}

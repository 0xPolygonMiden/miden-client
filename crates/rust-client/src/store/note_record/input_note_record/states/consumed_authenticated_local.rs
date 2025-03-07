use alloc::string::ToString;

use miden_objects::{
    Digest,
    block::BlockHeader,
    note::{NoteId, NoteInclusionProof, NoteMetadata},
    transaction::TransactionId,
};

use super::{InputNoteState, NoteStateHandler, NoteSubmissionData};
use crate::store::NoteRecordError;

/// Information related to notes in the [`InputNoteState::ConsumedAuthenticatedLocal`] state.
#[derive(Clone, Debug, PartialEq)]
pub struct ConsumedAuthenticatedLocalNoteState {
    /// Metadata associated with the note, including sender, note type, tag and other additional
    /// information.
    pub metadata: NoteMetadata,
    /// Inclusion proof for the note inside the chain block.
    pub inclusion_proof: NoteInclusionProof,
    /// Root of the note tree inside the block that verifies the note inclusion proof.
    pub block_note_root: Digest,
    /// Block height at which the note was nullified.
    pub nullifier_block_height: u32,
    /// Information about the submission of the note.
    pub submission_data: NoteSubmissionData,
}

impl NoteStateHandler for ConsumedAuthenticatedLocalNoteState {
    fn inclusion_proof_received(
        &self,
        _inclusion_proof: NoteInclusionProof,
        _metadata: NoteMetadata,
    ) -> Result<Option<InputNoteState>, NoteRecordError> {
        Ok(None)
    }

    fn consumed_externally(
        &self,
        _nullifier_block_height: u32,
    ) -> Result<Option<InputNoteState>, NoteRecordError> {
        Ok(None)
    }

    fn block_header_received(
        &self,
        _note_id: NoteId,
        _block_header: &BlockHeader,
    ) -> Result<Option<InputNoteState>, NoteRecordError> {
        Ok(None)
    }

    fn consumed_locally(
        &self,
        _consumer_account: miden_objects::account::AccountId,
        _consumer_transaction: miden_objects::transaction::TransactionId,
        _current_timestamp: Option<u64>,
    ) -> Result<Option<InputNoteState>, NoteRecordError> {
        Err(NoteRecordError::NoteNotConsumable("Note already consumed".to_string()))
    }

    fn transaction_committed(
        &self,
        _transaction_id: TransactionId,
        _block_height: u32,
    ) -> Result<Option<InputNoteState>, NoteRecordError> {
        Err(NoteRecordError::InvalidStateTransition(
            "Only processing notes can be committed in a local transaction".to_string(),
        ))
    }

    fn metadata(&self) -> Option<&NoteMetadata> {
        Some(&self.metadata)
    }

    fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        Some(&self.inclusion_proof)
    }

    fn consumer_transaction_id(&self) -> Option<&TransactionId> {
        Some(&self.submission_data.consumer_transaction)
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

impl From<ConsumedAuthenticatedLocalNoteState> for InputNoteState {
    fn from(state: ConsumedAuthenticatedLocalNoteState) -> Self {
        InputNoteState::ConsumedAuthenticatedLocal(state)
    }
}

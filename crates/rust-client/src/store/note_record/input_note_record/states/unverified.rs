use alloc::string::ToString;

use miden_objects::{
    block::{BlockHeader, BlockNumber},
    note::{compute_note_hash, NoteId, NoteInclusionProof, NoteMetadata},
    transaction::TransactionId,
};

use super::{
    CommittedNoteState, ConsumedExternalNoteState, InputNoteState, InvalidNoteState,
    NoteStateHandler, NoteSubmissionData, ProcessingUnauthenticatedNoteState,
};
use crate::store::NoteRecordError;

#[derive(Clone, Debug, PartialEq)]
pub struct UnverifiedNoteState {
    /// Metadata associated with the note, including sender, note type, tag and other additional
    /// information.
    pub metadata: NoteMetadata,
    /// Inclusion proof for the note inside the chain block. This proof isn't yet verified.
    pub inclusion_proof: NoteInclusionProof,
}

impl NoteStateHandler for UnverifiedNoteState {
    fn inclusion_proof_received(
        &self,
        inclusion_proof: NoteInclusionProof,
        metadata: NoteMetadata,
    ) -> Result<Option<InputNoteState>, NoteRecordError> {
        Ok(Some(UnverifiedNoteState { metadata, inclusion_proof }.into()))
    }

    fn consumed_externally(
        &self,
        nullifier_block_height: u32,
    ) -> Result<Option<InputNoteState>, NoteRecordError> {
        Ok(Some(ConsumedExternalNoteState { nullifier_block_height }.into()))
    }

    fn block_header_received(
        &self,
        note_id: NoteId,
        block_header: BlockHeader,
    ) -> Result<Option<InputNoteState>, NoteRecordError> {
        if self
            .inclusion_proof
            .note_path()
            .verify(
                self.inclusion_proof.location().node_index_in_block().into(),
                compute_note_hash(note_id, &self.metadata),
                &block_header.note_root(),
            )
            .is_ok()
        {
            Ok(Some(
                CommittedNoteState {
                    inclusion_proof: self.inclusion_proof.clone(),
                    metadata: self.metadata,
                    block_note_root: block_header.note_root(),
                }
                .into(),
            ))
        } else {
            Ok(Some(
                InvalidNoteState {
                    metadata: self.metadata,
                    invalid_inclusion_proof: self.inclusion_proof.clone(),
                    block_note_root: block_header.note_root(),
                }
                .into(),
            ))
        }
    }

    fn consumed_locally(
        &self,
        consumer_account: miden_objects::account::AccountId,
        consumer_transaction: miden_objects::transaction::TransactionId,
        _current_timestamp: Option<u64>,
    ) -> Result<Option<InputNoteState>, NoteRecordError> {
        let submission_data = NoteSubmissionData {
            submitted_at: None,
            consumer_account,
            consumer_transaction,
        };

        let after_block_num =
            self.inclusion_proof.location().block_num().as_u32().saturating_sub(1);
        Ok(Some(
            ProcessingUnauthenticatedNoteState {
                metadata: self.metadata,
                after_block_num: BlockNumber::from(after_block_num),
                submission_data,
            }
            .into(),
        ))
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
        None
    }
}

impl miden_tx::utils::Serializable for UnverifiedNoteState {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        self.metadata.write_into(target);
        self.inclusion_proof.write_into(target);
    }
}

impl miden_tx::utils::Deserializable for UnverifiedNoteState {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_tx::utils::DeserializationError> {
        let metadata = NoteMetadata::read_from(source)?;
        let inclusion_proof = NoteInclusionProof::read_from(source)?;
        Ok(UnverifiedNoteState { metadata, inclusion_proof })
    }
}

impl From<UnverifiedNoteState> for InputNoteState {
    fn from(state: UnverifiedNoteState) -> Self {
        InputNoteState::Unverified(state)
    }
}

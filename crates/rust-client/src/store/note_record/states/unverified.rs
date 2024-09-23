use miden_objects::{
    notes::{compute_note_hash, NoteId, NoteInclusionProof, NoteMetadata},
    BlockHeader,
};

use super::{
    CommittedNoteState, ConsumedExternalNoteState, InvalidNoteState, NoteState, NoteStateHandler,
    NoteSubmissionData, ProcessingUnauthenticatedNoteState,
};
use crate::store::NoteRecordError;

#[derive(Clone, Debug, PartialEq)]
pub struct UnverifiedNoteState {
    pub metadata: NoteMetadata,
    pub inclusion_proof: NoteInclusionProof,
}

impl NoteStateHandler for UnverifiedNoteState {
    fn inclusion_proof_received(
        &self,
        inclusion_proof: NoteInclusionProof,
        metadata: NoteMetadata,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        Ok(Some(UnverifiedNoteState { metadata, inclusion_proof }.into()))
    }

    fn nullifier_received(
        &self,
        nullifier_block_height: u32,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        Ok(Some(ConsumedExternalNoteState { nullifier_block_height }.into()))
    }

    fn block_header_received(
        &self,
        note_id: NoteId,
        block_header: BlockHeader,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        if self.inclusion_proof.note_path().verify(
            self.inclusion_proof.location().node_index_in_block().into(),
            compute_note_hash(note_id, &self.metadata),
            &block_header.note_root(),
        ) {
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
        consumer_account: miden_objects::accounts::AccountId,
        consumer_transaction: miden_objects::transaction::TransactionId,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        let submission_data = NoteSubmissionData {
            submitted_at: None,
            consumer_account,
            consumer_transaction,
        };

        Ok(Some(
            ProcessingUnauthenticatedNoteState {
                metadata: self.metadata,
                after_block_num: self.inclusion_proof.location().block_num() - 1,
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

impl From<UnverifiedNoteState> for NoteState {
    fn from(state: UnverifiedNoteState) -> Self {
        NoteState::Unverified(state)
    }
}

use alloc::string::ToString;

use miden_objects::{
    notes::{compute_note_hash, NoteId, NoteInclusionProof, NoteMetadata},
    BlockHeader, Digest,
};

use super::{
    CommittedNoteState, ConsumedExternalNoteState, NoteState, NoteStateHandler,
    UnverifiedNoteState, STATE_INVALID,
};
use crate::store::NoteRecordError;

#[derive(Clone, Debug, PartialEq)]
pub struct InvalidNoteState {
    pub metadata: NoteMetadata,
    pub invalid_inclusion_proof: NoteInclusionProof,
    pub block_note_root: Digest,
}

impl NoteStateHandler for InvalidNoteState {
    fn inclusion_proof_received(
        &self,
        inclusion_proof: NoteInclusionProof,
        metadata: NoteMetadata,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        Ok(Some(UnverifiedNoteState { inclusion_proof, metadata }.into()))
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
        if self.invalid_inclusion_proof.note_path().verify(
            self.invalid_inclusion_proof.location().node_index_in_block().into(),
            compute_note_hash(note_id, &self.metadata),
            &block_header.note_root(),
        ) {
            Ok(Some(
                CommittedNoteState {
                    inclusion_proof: self.invalid_inclusion_proof.clone(),
                    metadata: self.metadata,
                    block_note_root: block_header.note_root(),
                }
                .into(),
            ))
        } else {
            Ok(None)
        }
    }

    fn consumed_locally(
        &self,
        _consumer_account: miden_objects::accounts::AccountId,
        _consumer_transaction: miden_objects::transaction::TransactionId,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        Err(NoteRecordError::InvalidStateTransition {
            state: STATE_INVALID,
            transition_name: "consumed_locally".to_string(),
        })
    }

    fn metadata(&self) -> Option<&NoteMetadata> {
        Some(&self.metadata)
    }

    fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        Some(&self.invalid_inclusion_proof)
    }
}

impl miden_tx::utils::Serializable for InvalidNoteState {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        self.metadata.write_into(target);
        self.invalid_inclusion_proof.write_into(target);
        self.block_note_root.write_into(target);
    }
}

impl miden_tx::utils::Deserializable for InvalidNoteState {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_tx::utils::DeserializationError> {
        let metadata = NoteMetadata::read_from(source)?;
        let invalid_inclusion_proof = NoteInclusionProof::read_from(source)?;
        let block_note_root = Digest::read_from(source)?;
        Ok(InvalidNoteState {
            metadata,
            invalid_inclusion_proof,
            block_note_root,
        })
    }
}

impl From<InvalidNoteState> for NoteState {
    fn from(state: InvalidNoteState) -> Self {
        NoteState::Invalid(state)
    }
}

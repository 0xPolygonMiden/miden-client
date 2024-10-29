use alloc::string::ToString;

use miden_objects::{
    notes::{compute_note_hash, NoteId, NoteInclusionProof, NoteMetadata},
    transaction::TransactionId,
    BlockHeader, Digest,
};

use super::{
    CommittedNoteState, ConsumedExternalNoteState, InputNoteState, NoteStateHandler,
    UnverifiedNoteState,
};
use crate::store::NoteRecordError;

#[derive(Clone, Debug, PartialEq)]
pub struct InvalidNoteState {
    /// Metadata associated with the note, including sender, note type, tag and other additional
    /// information.
    pub metadata: NoteMetadata,
    /// Inclusion proof for the note inside the chain block.
    pub invalid_inclusion_proof: NoteInclusionProof,
    /// Root of the note tree inside the block that invalidates the note inclusion proof.
    pub block_note_root: Digest,
}

impl NoteStateHandler for InvalidNoteState {
    fn inclusion_proof_received(
        &self,
        inclusion_proof: NoteInclusionProof,
        metadata: NoteMetadata,
    ) -> Result<Option<InputNoteState>, NoteRecordError> {
        Ok(Some(UnverifiedNoteState { inclusion_proof, metadata }.into()))
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
            .invalid_inclusion_proof
            .note_path()
            .verify(
                self.invalid_inclusion_proof.location().node_index_in_block().into(),
                compute_note_hash(note_id, &self.metadata),
                &block_header.note_root(),
            )
            .is_ok()
        {
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
    ) -> Result<Option<InputNoteState>, NoteRecordError> {
        Err(NoteRecordError::NoteNotConsumable("Can't consume invalid note".to_string()))
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
        Some(&self.invalid_inclusion_proof)
    }

    fn consumer_transaction_id(&self) -> Option<&TransactionId> {
        None
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

impl From<InvalidNoteState> for InputNoteState {
    fn from(state: InvalidNoteState) -> Self {
        InputNoteState::Invalid(state)
    }
}

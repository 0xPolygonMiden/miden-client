use alloc::string::ToString;

use miden_objects::{
    accounts::AccountId,
    notes::{NoteId, NoteInclusionProof, NoteMetadata, NoteTag},
    transaction::TransactionId,
    BlockHeader,
};

use super::{
    ConsumedExternalNoteState, NoteState, NoteStateHandler, NoteSubmissionData,
    ProcessingUnauthenticatedNoteState, UnverifiedNoteState,
};
use crate::store::NoteRecordError;

#[derive(Clone, Debug, PartialEq)]
pub struct ExpectedNoteState {
    pub metadata: Option<NoteMetadata>,
    pub after_block_num: u32,
    pub tag: Option<NoteTag>,
}

impl NoteStateHandler for ExpectedNoteState {
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
        _note_id: NoteId,
        _block_header: BlockHeader,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        Err(NoteRecordError::StateTransitionError(
            "Can't verify an expected note".to_string(),
        ))
    }

    fn consumed_locally(
        &self,
        consumer_account: AccountId,
        consumer_transaction: TransactionId,
    ) -> Result<Option<NoteState>, NoteRecordError> {
        match self.metadata {
            None => Err(NoteRecordError::NoteNotConsumable(
                "Can't consume note without metadata".to_string(),
            )),
            Some(metadata) => {
                let submission_data = NoteSubmissionData {
                    submitted_at: None,
                    consumer_account,
                    consumer_transaction,
                };

                Ok(Some(
                    ProcessingUnauthenticatedNoteState {
                        metadata,
                        after_block_num: self.after_block_num,
                        submission_data,
                    }
                    .into(),
                ))
            },
        }
    }

    fn metadata(&self) -> Option<&NoteMetadata> {
        self.metadata.as_ref()
    }

    fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        None
    }
}

impl miden_tx::utils::Serializable for ExpectedNoteState {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        self.metadata.write_into(target);
        self.after_block_num.write_into(target);
        self.tag.write_into(target);
    }
}

impl miden_tx::utils::Deserializable for ExpectedNoteState {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_tx::utils::DeserializationError> {
        let metadata = Option::<NoteMetadata>::read_from(source)?;
        let after_block_num = u32::read_from(source)?;
        let tag = Option::<NoteTag>::read_from(source)?;
        Ok(ExpectedNoteState { metadata, after_block_num, tag })
    }
}

impl From<ExpectedNoteState> for NoteState {
    fn from(state: ExpectedNoteState) -> Self {
        NoteState::Expected(state)
    }
}

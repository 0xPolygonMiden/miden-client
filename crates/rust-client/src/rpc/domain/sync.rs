use alloc::vec::Vec;

use miden_objects::{
    Digest,
    account::AccountId,
    block::{BlockHeader, BlockNumber},
    crypto::merkle::MmrDelta,
    note::NoteId,
    transaction::TransactionId,
};

use super::{note::CommittedNote, transaction::TransactionInclusion};
use crate::rpc::{RpcError, generated::responses::SyncStateResponse};

// STATE SYNC INFO
// ================================================================================================

/// Represents a `SyncStateResponse` with fields converted into domain types.
pub struct StateSyncInfo {
    /// The block number of the chain tip at the moment of the response.
    pub chain_tip: BlockNumber,
    /// The returned block header.
    pub block_header: BlockHeader,
    /// MMR delta that contains data for (`current_block.num`, `incoming_block_header.num-1`).
    pub mmr_delta: MmrDelta,
    /// Tuples of `AccountId` alongside their new account commitments.
    pub account_commitment_updates: Vec<(AccountId, Digest)>,
    /// List of tuples of Note ID, Note Index and Merkle Path for all new notes.
    pub note_inclusions: Vec<CommittedNote>,
    /// List of transaction IDs of transaction that were included in (`request.block_num`,
    /// `response.block_num-1`) along with the account the tx was executed against and the block
    /// number the transaction was included in.
    pub transactions: Vec<TransactionInclusion>,
}

// STATE SYNC INFO CONVERSION
// ================================================================================================

impl TryFrom<SyncStateResponse> for StateSyncInfo {
    type Error = RpcError;

    fn try_from(value: SyncStateResponse) -> Result<Self, Self::Error> {
        let chain_tip = value.chain_tip;

        // Validate and convert block header
        let block_header: BlockHeader = value
            .block_header
            .ok_or(RpcError::ExpectedDataMissing("BlockHeader".into()))?
            .try_into()?;

        // Validate and convert MMR Delta
        let mmr_delta = value
            .mmr_delta
            .ok_or(RpcError::ExpectedDataMissing("MmrDelta".into()))?
            .try_into()?;

        // Validate and convert account commitment updates into an (AccountId, Digest) tuple
        let mut account_commitment_updates = vec![];
        for update in value.accounts {
            let account_id = update
                .account_id
                .ok_or(RpcError::ExpectedDataMissing("AccountCommitmentUpdate.AccountId".into()))?
                .try_into()?;
            let account_commitment = update
                .account_commitment
                .ok_or(RpcError::ExpectedDataMissing(
                    "AccountCommitmentUpdate.AccountCommitment".into(),
                ))?
                .try_into()?;
            account_commitment_updates.push((account_id, account_commitment));
        }

        // Validate and convert account note inclusions into an (AccountId, Digest) tuple
        let mut note_inclusions = vec![];
        for note in value.notes {
            let note_id: Digest = note
                .note_id
                .ok_or(RpcError::ExpectedDataMissing("Notes.Id".into()))?
                .try_into()?;

            let note_id: NoteId = note_id.into();

            let merkle_path = note
                .merkle_path
                .ok_or(RpcError::ExpectedDataMissing("Notes.MerklePath".into()))?
                .try_into()?;

            let metadata = note
                .metadata
                .ok_or(RpcError::ExpectedDataMissing("Metadata".into()))?
                .try_into()?;

            let committed_note = super::note::CommittedNote::new(
                note_id,
                u16::try_from(note.note_index).expect("note index out of range"),
                merkle_path,
                metadata,
            );

            note_inclusions.push(committed_note);
        }

        let transactions = value
            .transactions
            .iter()
            .map(|transaction_summary| {
                let transaction_id = transaction_summary.transaction_id.ok_or(
                    RpcError::ExpectedDataMissing("TransactionSummary.TransactionId".into()),
                )?;
                let transaction_id = TransactionId::try_from(transaction_id)?;

                let transaction_block_num = transaction_summary.block_num;

                let transaction_account_id = transaction_summary.account_id.clone().ok_or(
                    RpcError::ExpectedDataMissing("TransactionSummary.TransactionId".into()),
                )?;
                let transaction_account_id = AccountId::try_from(transaction_account_id)?;

                Ok(TransactionInclusion {
                    transaction_id,
                    block_num: transaction_block_num,
                    account_id: transaction_account_id,
                })
            })
            .collect::<Result<Vec<TransactionInclusion>, RpcError>>()?;

        Ok(Self {
            chain_tip: chain_tip.into(),
            block_header,
            mmr_delta,
            account_commitment_updates,
            note_inclusions,
            transactions,
        })
    }
}

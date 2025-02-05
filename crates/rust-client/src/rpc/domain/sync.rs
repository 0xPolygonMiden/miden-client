use alloc::vec::Vec;

use miden_objects::{
    account::AccountId,
    block::{BlockHeader, BlockNumber},
    crypto::merkle::MmrDelta,
    note::NoteId,
    transaction::TransactionId,
    Digest,
};

use super::{note::CommittedNote, nullifier::NullifierUpdate, transaction::TransactionUpdate};
use crate::rpc::{generated::responses::SyncStateResponse, RpcError};

// STATE SYNC INFO
// ================================================================================================

/// Represents a `SyncStateResponse` with fields converted into domain types.
pub struct StateSyncInfo {
    /// The block number of the chain tip at the moment of the response.
    pub chain_tip: BlockNumber,
    /// The returned block header.
    pub block_header: BlockHeader,
    /// MMR delta that contains data for `(current_block.num, incoming_block_header.num-1)`.
    pub mmr_delta: MmrDelta,
    /// Tuples of `AccountId` alongside their new account hashes.
    pub account_hash_updates: Vec<(AccountId, Digest)>,
    /// List of tuples of Note ID, Note Index and Merkle Path for all new notes.
    pub note_inclusions: Vec<CommittedNote>,
    /// List of nullifiers that identify spent notes along with the block number at which they were
    /// consumed.
    pub nullifiers: Vec<NullifierUpdate>,
    /// List of transaction IDs of transaction that were included in (`request.block_num`,
    /// `response.block_num-1`) along with the account the tx was executed against and the block
    /// number the transaction was included in.
    pub transactions: Vec<TransactionUpdate>,
}

// STATE SYNC INFO CONVERSION
// ================================================================================================

impl TryFrom<SyncStateResponse> for StateSyncInfo {
    type Error = RpcError;

    #[allow(clippy::cast_possible_truncation)]
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

        // Validate and convert account hash updates into an (AccountId, Digest) tuple
        let mut account_hash_updates = vec![];
        for update in value.accounts {
            let account_id = update
                .account_id
                .ok_or(RpcError::ExpectedDataMissing("AccountHashUpdate.AccountId".into()))?
                .try_into()?;
            let account_hash = update
                .account_hash
                .ok_or(RpcError::ExpectedDataMissing("AccountHashUpdate.AccountHash".into()))?
                .try_into()?;
            account_hash_updates.push((account_id, account_hash));
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
                note.note_index as u16,
                merkle_path,
                metadata,
            );

            note_inclusions.push(committed_note);
        }

        let nullifiers = value
            .nullifiers
            .iter()
            .map(|nul_update| {
                let nullifier_digest = nul_update
                    .nullifier
                    .ok_or(RpcError::ExpectedDataMissing("Nullifier".into()))?;

                let nullifier_digest = Digest::try_from(nullifier_digest)?;

                let nullifier_block_num = nul_update.block_num;

                Ok(NullifierUpdate {
                    nullifier: nullifier_digest.into(),
                    block_num: nullifier_block_num,
                })
            })
            .collect::<Result<Vec<NullifierUpdate>, RpcError>>()?;

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

                Ok(TransactionUpdate {
                    transaction_id,
                    block_num: transaction_block_num,
                    account_id: transaction_account_id,
                })
            })
            .collect::<Result<Vec<TransactionUpdate>, RpcError>>()?;

        Ok(Self {
            chain_tip: chain_tip.into(),
            block_header,
            mmr_delta,
            account_hash_updates,
            note_inclusions,
            nullifiers,
            transactions,
        })
    }
}

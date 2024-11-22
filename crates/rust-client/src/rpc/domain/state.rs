use alloc::vec::Vec;

use miden_objects::{accounts::AccountId, crypto::merkle::MmrDelta, BlockHeader, Digest};

use super::{notes::CommittedNote, nullifiers::NullifierUpdate, transactions::TransactionUpdate};

// STATE SYNC INFO
// ================================================================================================

/// Represents a `SyncStateResponse` with fields converted into domain types.
pub struct StateSyncInfo {
    /// The block number of the chain tip at the moment of the response.
    pub chain_tip: u32,
    /// The returned block header.
    pub block_header: BlockHeader,
    /// MMR delta that contains data for (current_block.num, incoming_block_header.num-1).
    pub mmr_delta: MmrDelta,
    /// Tuples of AccountId alongside their new account hashes.
    pub account_hash_updates: Vec<(AccountId, Digest)>,
    /// List of tuples of Note ID, Note Index and Merkle Path for all new notes.
    pub note_inclusions: Vec<CommittedNote>,
    /// List of nullifiers that identify spent notes along with the block number at which they were
    /// consumed.
    pub nullifiers: Vec<NullifierUpdate>,
    /// List of transaction IDs of transaction that were included in (request.block_num,
    /// response.block_num-1) along with the account the tx was executed against and the block
    /// number the transaction was included in.
    pub transactions: Vec<TransactionUpdate>,
}

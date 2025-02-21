use alloc::vec::Vec;

use miden_objects::{
    account::Account,
    block::BlockHeader,
    crypto::merkle::{InOrderIndex, MmrPeaks},
    Digest,
};

use super::{NoteTagRecord, SyncSummary};
use crate::{note::NoteUpdates, store::AccountUpdates, transaction::TransactionUpdates};

// STATE SYNC UPDATE
// ================================================================================================

/// Contains all information needed to apply the update in the store after syncing with the node.
pub struct StateSyncUpdate {
    /// The new block header, returned as part of the
    /// [`StateSyncInfo`](crate::rpc::domain::sync::StateSyncInfo)
    pub block_header: BlockHeader,
    /// Whether the block header has notes relevant to the client.
    pub block_has_relevant_notes: bool,
    /// New MMR peaks for the locally tracked MMR of the blockchain.
    pub new_mmr_peaks: MmrPeaks,
    /// New authentications nodes that are meant to be stored in order to authenticate block
    /// headers.
    pub new_authentication_nodes: Vec<(InOrderIndex, Digest)>,
    /// New and updated notes to be upserted in the store.
    pub note_updates: NoteUpdates,
    /// Committed and discarded transactions after the sync.
    pub transaction_updates: TransactionUpdates,
    /// Public account updates and mismatched private accounts after the sync.
    pub account_updates: AccountUpdates,
    /// Tag records that are no longer relevant.
    pub tags_to_remove: Vec<NoteTagRecord>,
}

impl From<&StateSyncUpdate> for SyncSummary {
    fn from(value: &StateSyncUpdate) -> Self {
        SyncSummary::new(
            value.block_header.block_num(),
            value.note_updates.committed_note_ids().into_iter().collect(),
            value.note_updates.consumed_note_ids().into_iter().collect(),
            value
                .account_updates
                .updated_public_accounts()
                .iter()
                .map(Account::id)
                .collect(),
            value
                .account_updates
                .mismatched_private_accounts()
                .iter()
                .map(|(id, _)| *id)
                .collect(),
            value
                .transaction_updates
                .committed_transactions()
                .iter()
                .map(|t| t.transaction_id)
                .collect(),
        )
    }
}

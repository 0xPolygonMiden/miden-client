use alloc::{collections::BTreeSet, vec::Vec};

use miden_objects::{
    Digest,
    account::AccountId,
    block::{BlockHeader, BlockNumber},
    crypto::merkle::{InOrderIndex, MmrPeaks},
    note::NoteId,
    transaction::TransactionId,
};

use super::SyncSummary;
use crate::{
    account::Account,
    note::{NoteUpdateTracker, NoteUpdateType},
    rpc::domain::transaction::TransactionUpdate,
    transaction::TransactionRecord,
};

// STATE SYNC UPDATE
// ================================================================================================

/// Contains all information needed to apply the update in the store after syncing with the node.
#[derive(Default)]
pub struct StateSyncUpdate {
    /// The block number of the last block that was synced.
    pub block_num: BlockNumber,
    /// New blocks and authentication nodes.
    pub block_updates: BlockUpdates,
    /// New and updated notes to be upserted in the store.
    pub note_updates: NoteUpdateTracker,
    /// Committed and discarded transactions after the sync.
    pub transaction_updates: TransactionUpdates,
    /// Public account updates and mismatched private accounts after the sync.
    pub account_updates: AccountUpdates,
}

impl From<&StateSyncUpdate> for SyncSummary {
    fn from(value: &StateSyncUpdate) -> Self {
        let new_public_note_ids = value
            .note_updates
            .updated_input_notes()
            .filter_map(|note_update| {
                let note = note_update.inner();
                if let NoteUpdateType::Insert = note_update.update_type() {
                    Some(note.id())
                } else {
                    None
                }
            })
            .collect();

        let committed_note_ids: BTreeSet<NoteId> = value
            .note_updates
            .updated_input_notes()
            .filter_map(|note_update| {
                let note = note_update.inner();
                if let NoteUpdateType::Update = note_update.update_type() {
                    note.is_committed().then_some(note.id())
                } else {
                    None
                }
            })
            .chain(value.note_updates.updated_output_notes().filter_map(|note_update| {
                let note = note_update.inner();
                if let NoteUpdateType::Update = note_update.update_type() {
                    note.is_committed().then_some(note.id())
                } else {
                    None
                }
            }))
            .collect();

        let consumed_note_ids: BTreeSet<NoteId> = value
            .note_updates
            .updated_input_notes()
            .filter_map(|note| note.inner().is_consumed().then_some(note.inner().id()))
            .collect();

        SyncSummary::new(
            value.block_num,
            new_public_note_ids,
            committed_note_ids.into_iter().collect(),
            consumed_note_ids.into_iter().collect(),
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

/// Contains all the block information that needs to be added in the client's store after a sync.
#[derive(Debug, Clone, Default)]
pub struct BlockUpdates {
    /// New block headers to be stored, along with a flag indicating whether the block contains
    /// notes that are relevant to the client and the MMR peaks for the block.
    block_headers: Vec<(BlockHeader, bool, MmrPeaks)>,
    /// New authentication nodes that are meant to be stored in order to authenticate block
    /// headers.
    new_authentication_nodes: Vec<(InOrderIndex, Digest)>,
}

impl BlockUpdates {
    /// Creates a new instance of [`BlockUpdates`].
    pub fn new(
        block_headers: Vec<(BlockHeader, bool, MmrPeaks)>,
        new_authentication_nodes: Vec<(InOrderIndex, Digest)>,
    ) -> Self {
        Self { block_headers, new_authentication_nodes }
    }

    /// Returns the new block headers to be stored, along with a flag indicating whether the block
    /// contains notes that are relevant to the client and the MMR peaks for the block.
    pub fn block_headers(&self) -> &[(BlockHeader, bool, MmrPeaks)] {
        &self.block_headers
    }

    /// Returns the new authentication nodes that are meant to be stored in order to authenticate
    /// block headers.
    pub fn new_authentication_nodes(&self) -> &[(InOrderIndex, Digest)] {
        &self.new_authentication_nodes
    }

    /// Extends the current [`BlockUpdates`] with the provided one.
    pub(crate) fn extend(&mut self, other: BlockUpdates) {
        self.block_headers.extend(other.block_headers);
        self.new_authentication_nodes.extend(other.new_authentication_nodes);
    }
}

/// Contains transaction changes to apply to the store.
#[derive(Default)]
pub struct TransactionUpdates {
    /// Transaction updates for any transaction that was committed between the sync request's block
    /// number and the response's block number.
    committed_transactions: Vec<TransactionUpdate>,
    /// Transaction IDs for any transactions that were discarded in the sync.
    discarded_transactions: Vec<TransactionId>,
    /// Transactions that were pending before the sync and were not committed.
    ///
    /// These transactions have been pending for more than [`TX_GRACEFUL_BLOCKS`] blocks and can be
    /// assumed to have been rejected by the network. They will be marked as discarded in the
    /// store.
    stale_transactions: Vec<TransactionRecord>,
}

impl TransactionUpdates {
    /// Creates a new [`TransactionUpdate`]
    pub fn new(
        committed_transactions: Vec<TransactionUpdate>,
        discarded_transactions: Vec<TransactionId>,
        stale_transactions: Vec<TransactionRecord>,
    ) -> Self {
        Self {
            committed_transactions,
            discarded_transactions,
            stale_transactions,
        }
    }

    /// Extends the transaction update information with `other`.
    pub fn extend(&mut self, other: Self) {
        self.committed_transactions.extend(other.committed_transactions);
        self.discarded_transactions.extend(other.discarded_transactions);
        self.stale_transactions.extend(other.stale_transactions);
    }

    /// Returns a reference to committed transactions.
    pub fn committed_transactions(&self) -> &[TransactionUpdate] {
        &self.committed_transactions
    }

    /// Returns a reference to discarded transactions.
    pub fn discarded_transactions(&self) -> &[TransactionId] {
        &self.discarded_transactions
    }

    /// Returns a reference to stale transactions.
    pub fn stale_transactions(&self) -> &[TransactionRecord] {
        &self.stale_transactions
    }
}

// ACCOUNT UPDATES
// ================================================================================================

/// Contains account changes to apply to the store after a sync request.
#[derive(Debug, Clone, Default)]
pub struct AccountUpdates {
    /// Updated public accounts.
    updated_public_accounts: Vec<Account>,
    /// Account commitments received from the network that don't match the currently
    /// locally-tracked state of the private accounts.
    ///
    /// These updates may represent a stale account commitment (meaning that the latest local state
    /// hasn't been committed). If this is not the case, the account may be locked until the state
    /// is restored manually.
    mismatched_private_accounts: Vec<(AccountId, Digest)>,
}

impl AccountUpdates {
    /// Creates a new instance of `AccountUpdates`.
    pub fn new(
        updated_public_accounts: Vec<Account>,
        mismatched_private_accounts: Vec<(AccountId, Digest)>,
    ) -> Self {
        Self {
            updated_public_accounts,
            mismatched_private_accounts,
        }
    }

    /// Returns the updated public accounts.
    pub fn updated_public_accounts(&self) -> &[Account] {
        &self.updated_public_accounts
    }

    /// Returns the mismatched private accounts.
    pub fn mismatched_private_accounts(&self) -> &[(AccountId, Digest)] {
        &self.mismatched_private_accounts
    }

    pub fn extend(&mut self, other: AccountUpdates) {
        self.updated_public_accounts.extend(other.updated_public_accounts);
        self.mismatched_private_accounts.extend(other.mismatched_private_accounts);
    }
}

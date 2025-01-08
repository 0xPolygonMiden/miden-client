//! Provides the client APIs for synchronizing the client's local state with the Miden
//! rollup network. It ensures that the client maintains a valid, up-to-date view of the chain.

use alloc::vec::Vec;
use core::cmp::max;
use std::boxed::Box;

use miden_objects::{
    accounts::AccountId,
    crypto::rand::FeltRng,
    notes::{NoteId, NoteTag, Nullifier},
    transaction::TransactionId,
};

use crate::{notes::NoteUpdates, Client, ClientError};

mod block_headers;

mod tags;
pub use tags::{NoteTagRecord, NoteTagSource};

mod state_sync;
pub use state_sync::{
    account_state_sync, committed_note_updates, consumed_note_updates, StateSync, StateSyncUpdate,
    SyncStatus,
};

/// Contains stats about the sync operation.
pub struct SyncSummary {
    /// Block number up to which the client has been synced.
    pub block_num: u32,
    /// IDs of new notes received.
    pub received_notes: Vec<NoteId>,
    /// IDs of tracked notes that received inclusion proofs.
    pub committed_notes: Vec<NoteId>,
    /// IDs of notes that have been consumed.
    pub consumed_notes: Vec<NoteId>,
    /// IDs of on-chain accounts that have been updated.
    pub updated_accounts: Vec<AccountId>,
    /// IDs of private accounts that have been locked.
    pub locked_accounts: Vec<AccountId>,
    /// IDs of committed transactions.
    pub committed_transactions: Vec<TransactionId>,
}

impl SyncSummary {
    pub fn new(
        block_num: u32,
        received_notes: Vec<NoteId>,
        committed_notes: Vec<NoteId>,
        consumed_notes: Vec<NoteId>,
        updated_accounts: Vec<AccountId>,
        locked_accounts: Vec<AccountId>,
        committed_transactions: Vec<TransactionId>,
    ) -> Self {
        Self {
            block_num,
            received_notes,
            committed_notes,
            consumed_notes,
            updated_accounts,
            locked_accounts,
            committed_transactions,
        }
    }

    pub fn new_empty(block_num: u32) -> Self {
        Self {
            block_num,
            received_notes: vec![],
            committed_notes: vec![],
            consumed_notes: vec![],
            updated_accounts: vec![],
            locked_accounts: vec![],
            committed_transactions: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.received_notes.is_empty()
            && self.committed_notes.is_empty()
            && self.consumed_notes.is_empty()
            && self.updated_accounts.is_empty()
            && self.locked_accounts.is_empty()
    }

    pub fn combine_with(&mut self, mut other: Self) {
        self.block_num = max(self.block_num, other.block_num);
        self.received_notes.append(&mut other.received_notes);
        self.committed_notes.append(&mut other.committed_notes);
        self.consumed_notes.append(&mut other.consumed_notes);
        self.updated_accounts.append(&mut other.updated_accounts);
        self.locked_accounts.append(&mut other.locked_accounts);
    }
}

// CONSTANTS
// ================================================================================================

/// The number of bits to shift identifiers for in use of filters.
pub(crate) const FILTER_ID_SHIFT: u8 = 48;

/// Client syncronization methods.
impl<R: FeltRng> Client<R> {
    // SYNC STATE
    // --------------------------------------------------------------------------------------------

    /// Returns the block number of the last state sync block.
    pub async fn get_sync_height(&self) -> Result<u32, ClientError> {
        self.store.get_sync_height().await.map_err(|err| err.into())
    }

    /// Syncs the client's state with the current state of the Miden network. Returns the block
    /// number the client has been synced to.
    ///
    /// The sync process is done in multiple steps:
    /// 1. A request is sent to the node to get the state updates. This request includes tracked
    ///    account IDs and the tags of notes that might have changed or that might be of interest to
    ///    the client.
    /// 2. A response is received with the current state of the network. The response includes
    ///    information about new/committed/consumed notes, updated accounts, and committed
    ///    transactions.
    /// 3. Tracked notes are updated with their new states.
    /// 4. New notes are checked, and only relevant ones are stored. Relevant notes are those that
    ///    can be consumed by accounts the client is tracking (this is checked by the
    ///    [crate::notes::NoteScreener])
    /// 5. Transactions are updated with their new states.
    /// 6. Tracked public accounts are updated and off-chain accounts are validated against the node
    ///    state.
    /// 7. The MMR is updated with the new peaks and authentication nodes.
    /// 8. All updates are applied to the store to be persisted.
    pub async fn sync_state(&mut self) -> Result<SyncSummary, ClientError> {
        _ = self.ensure_genesis_in_place().await?;

        let state_sync = StateSync::new(
            self.rpc_api.clone(),
            Box::new({
                let store_clone = self.store.clone();
                let rpc_api_clone = self.rpc_api.clone();
                move |committed_notes, block_header| {
                    Box::pin(committed_note_updates(
                        store_clone.clone(),
                        rpc_api_clone.clone(),
                        committed_notes,
                        block_header,
                    ))
                }
            }),
            Box::new({
                let store_clone = self.store.clone();
                move |nullifier_updates, committed_transactions| {
                    Box::pin(consumed_note_updates(
                        store_clone.clone(),
                        nullifier_updates,
                        committed_transactions,
                    ))
                }
            }),
            Box::new({
                let store_clone = self.store.clone();
                let rpc_api_clone = self.rpc_api.clone();
                move |account_hash_updates| {
                    Box::pin(account_state_sync(
                        store_clone.clone(),
                        rpc_api_clone.clone(),
                        account_hash_updates,
                    ))
                }
            }),
        );

        let current_block_num = self.store.get_sync_height().await?;
        let mut total_sync_summary = SyncSummary::new_empty(current_block_num);

        loop {
            // Get current state of the client
            let current_block_num = self.store.get_sync_height().await?;
            let (current_block, has_relevant_notes) =
                self.store.get_block_header_by_num(current_block_num).await?;

            let account_ids: Vec<AccountId> = self
                .store
                .get_account_headers()
                .await?
                .into_iter()
                .map(|(acc_header, _)| acc_header.id())
                .collect();

            let note_tags: Vec<NoteTag> =
                self.store.get_unique_note_tags().await?.into_iter().collect();

            let unspent_nullifiers = self.store.get_unspent_input_note_nullifiers().await?;

            // Get the sync update from the network
            let status = state_sync
                .sync_state_step(
                    current_block,
                    has_relevant_notes,
                    self.build_current_partial_mmr(false).await?,
                    account_ids,
                    note_tags,
                    unspent_nullifiers,
                )
                .await?;

            let (is_last_block, state_sync_update) = if let Some(status) = status {
                (
                    matches!(status, SyncStatus::SyncedToLastBlock(_)),
                    status.into_state_sync_update(),
                )
            } else {
                break;
            };

            let sync_summary: SyncSummary = (&state_sync_update).into();

            let has_relevant_notes =
                self.check_block_relevance(&state_sync_update.note_updates).await?;

            // Apply received and computed updates to the store
            self.store
                .apply_state_sync_step(state_sync_update, has_relevant_notes)
                .await
                .map_err(ClientError::StoreError)?;

            total_sync_summary.combine_with(sync_summary);

            if is_last_block {
                break;
            }
        }
        self.update_mmr_data().await?;

        Ok(total_sync_summary)
    }
}

pub(crate) fn get_nullifier_prefix(nullifier: &Nullifier) -> u16 {
    (nullifier.inner()[3].as_int() >> FILTER_ID_SHIFT) as u16
}

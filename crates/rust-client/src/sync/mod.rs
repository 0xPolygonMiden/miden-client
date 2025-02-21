//! Provides the client APIs for synchronizing the client's local state with the Miden
//! network. It ensures that the client maintains a valid, up-to-date view of the chain.
//!
//! ## Overview
//!
//! This module handles the synchronization process between the local client and the Miden network.
//! The sync operation involves:
//!
//! - Querying the Miden node for state updates using tracked account IDs, note tags, and nullifier
//!   prefixes.
//! - Processing the received data to update note inclusion proofs, reconcile note state (new,
//!   committed, or consumed), and update account states.
//! - Incorporating new block headers and updating the local Merkle Mountain Range (MMR) with new
//!   peaks and authentication nodes.
//! - Aggregating transaction updates to determine which transactions have been committed or
//!   discarded.
//!
//! The result of the synchronization process is captured in a [`SyncSummary`], which provides
//! a summary of the new block number along with lists of received, committed, and consumed note
//! IDs, updated account IDs, locked accounts, and committed transaction IDs.
//!
//! Once the data is requested and retrieved, updates are persisted in the client's store.
//!
//! ## Examples
//!
//! The following example shows how to initiate a state sync and handle the resulting summary:
//!
//! ```rust
//! # use miden_client::sync::SyncSummary;
//! # use miden_client::{Client, ClientError};
//! # use miden_objects::{block::BlockHeader, Felt, Word, StarkField};
//! # use miden_objects::crypto::rand::FeltRng;
//! # async fn run_sync<R: FeltRng>(client: &mut Client<R>) -> Result<(), ClientError> {
//! // Attempt to synchronize the client's state with the Miden network.
//! // The requested data is based on the client's state: it gets updates for accounts, relevant
//! // notes, etc. For more information on the data that gets requested, see the doc comments for
//! // `sync_state()`.
//! let sync_summary: SyncSummary = client.sync_state().await?;
//!
//! println!("Synced up to block number: {}", sync_summary.block_num);
//! println!("Committed notes: {}", sync_summary.committed_notes.len());
//! println!("Consumed notes: {}", sync_summary.consumed_notes.len());
//! println!("Updated accounts: {}", sync_summary.updated_accounts.len());
//! println!("Locked accounts: {}", sync_summary.locked_accounts.len());
//! println!("Committed transactions: {}", sync_summary.committed_transactions.len());
//!
//! Ok(())
//! # }
//! ```
//!
//! The `sync_state` method loops internally until the client is fully synced to the network tip.
//!
//! For more advanced usage, refer to the individual functions (such as
//! `committed_note_updates` and `consumed_note_updates`) to understand how the sync data is
//! processed and applied to the local store.

use alloc::{boxed::Box, vec::Vec};
use core::cmp::max;

use miden_objects::{
    account::AccountId,
    block::BlockNumber,
    crypto::rand::FeltRng,
    note::{NoteId, NoteTag, Nullifier},
    transaction::TransactionId,
};
use state_sync::on_note_received;

use crate::{
    note::NoteUpdates,
    rpc::{
        domain::{
            note::CommittedNote, nullifier::NullifierUpdate, sync::StateSyncInfo,
            transaction::TransactionUpdate,
        },
        RpcError::ConnectionError,
    },
    store::{AccountUpdates, InputNoteRecord, NoteFilter, OutputNoteRecord, TransactionFilter},
    Client, ClientError,
};

mod block_header;

mod tag;
pub use tag::{NoteTagRecord, NoteTagSource};

mod state_sync;
pub use state_sync::{OnNoteReceived, OnNullifierReceived, StateSync};

impl SyncSummary {
    pub fn new(
        block_num: BlockNumber,
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

    pub fn new_empty(block_num: BlockNumber) -> Self {
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

/// Contains all information needed to apply the update in the store after syncing with the node.
pub struct StateSyncUpdate {
    /// The new block header, returned as part of the [`StateSyncInfo`]
    pub block_header: BlockHeader,
    /// Information about note changes after the sync.
    pub note_updates: NoteUpdates,
    /// Transaction updates for any transaction that was committed between the sync request's
    /// block number and the response's block number.
    pub transactions_to_commit: Vec<TransactionUpdate>,
    /// Transaction IDs for any transactions that were discarded in the sync.
    pub transactions_to_discard: Vec<TransactionId>,
    /// New MMR peaks for the locally tracked MMR of the blockchain.
    pub new_mmr_peaks: MmrPeaks,
    /// New authentications nodes that are meant to be stored in order to authenticate block
    /// headers.
    pub new_authentication_nodes: Vec<(InOrderIndex, Digest)>,
    /// Information abount account changes after the sync.
    pub updated_accounts: AccountUpdates,
    /// Whether the block header has notes relevant to the client.
    pub block_has_relevant_notes: bool,
    /// Tag records that are no longer relevant.
    pub tags_to_remove: Vec<NoteTagRecord>,
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
    pub async fn get_sync_height(&self) -> Result<BlockNumber, ClientError> {
        self.store.get_sync_height().await.map_err(Into::into)
    }

    /// Syncs the client's state with the current state of the Miden network and returns a
    /// [`SyncSummary`] corresponding to the local state update.
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
    ///    [`crate::note::NoteScreener`])
    /// 5. Transactions are updated with their new states.
    /// 6. Tracked public accounts are updated and private accounts are validated against the node
    ///    state.
    /// 7. The MMR is updated with the new peaks and authentication nodes.
    /// 8. All updates are applied to the store to be persisted.
    pub async fn sync_state(&mut self) -> Result<SyncSummary, ClientError> {
        _ = self.ensure_genesis_in_place().await?;

        let current_block_num = self.store.get_sync_height().await?;
        let mut total_sync_summary = SyncSummary::new_empty(current_block_num);

        let accounts: Vec<AccountHeader> = self
            .store
            .get_account_headers()
            .await?
            .into_iter()
            .map(|(acc_header, _)| acc_header)
            .collect();

        let note_tags: Vec<NoteTag> =
            self.store.get_unique_note_tags().await?.into_iter().collect();

        let unspent_input_notes = self.store.get_input_notes(NoteFilter::Unspent).await?;
        let unspent_output_notes = self.store.get_output_notes(NoteFilter::Unspent).await?;

        let account_ids: Vec<AccountId> = accounts.iter().map(AccountHeader::id).collect();

        let mut response_stream = self
            .rpc_api
            .sync_state(current_block_num, &account_ids, &note_tags, &nullifiers_tags)
            .await?;

        loop {
            let response = response_stream
                .message()
                .await
                .map_err(|e| ClientError::RpcError(ConnectionError(e.message().to_string())))?;

            let response: StateSyncInfo = match response {
                Some(res) => res.try_into().map_err(ClientError::RpcError)?,
                None => {
                    // Stream closed, sync completed
                    break;
                },
            };

            let summary = self
                .apply_state_sync(response, total_sync_summary.block_num, accounts.clone())
                .await?;

            total_sync_summary.combine_with(summary);
        }
        self.update_mmr_data().await?;

        Ok(total_sync_summary)
    }

    async fn apply_state_sync(
        &mut self,
        response: StateSyncInfo,
        current_block_num: BlockNumber,
        accounts: Vec<AccountHeader>,
    ) -> Result<SyncSummary, ClientError> {
        let (committed_note_updates, tags_to_remove) = self
            .committed_note_updates(response.note_inclusions, &response.block_header)
            .await?;

        let incoming_block_has_relevant_notes =
            self.check_block_relevance(&committed_note_updates).await?;

        let transactions_to_commit = self.get_transactions_to_commit(response.transactions).await?;

        let (consumed_note_updates, transactions_to_discard) =
            self.consumed_note_updates(response.nullifiers, &transactions_to_commit).await?;

        let note_updates = committed_note_updates.combine_with(consumed_note_updates);

        let (public_accounts, private_accounts): (Vec<_>, Vec<_>) =
            accounts.into_iter().partition(|account_header| account_header.id().is_public());

        let updated_public_accounts = self
            .get_updated_public_accounts(&response.account_hash_updates, &public_accounts)
            .await?;

        let mismatched_private_accounts = self
            .validate_local_account_hashes(&response.account_hash_updates, &private_accounts)
            .await?;

        // Build PartialMmr with current data and apply updates
        let (new_peaks, new_authentication_nodes) = {
            let current_partial_mmr = self.build_current_partial_mmr(false).await?;

            let (current_block, has_relevant_notes) = self
                .store
                .get_block_header_by_num(current_block_num)
                .await?
                .expect("Current block should be in the store");

            apply_mmr_changes(
                current_partial_mmr,
                response.mmr_delta,
                &current_block,
                has_relevant_notes,
            )?
        };

        // Store summary to return later
        let sync_summary = SyncSummary::new(
            response.block_header.block_num(),
            note_updates.new_input_notes().iter().map(InputNoteRecord::id).collect(),
            note_updates.committed_note_ids().into_iter().collect(),
            note_updates.consumed_note_ids().into_iter().collect(),
            updated_public_accounts.iter().map(Account::id).collect(),
            mismatched_private_accounts.iter().map(|(acc_id, _)| *acc_id).collect(),
            transactions_to_commit.iter().map(|tx| tx.transaction_id).collect(),
        );

        let state_sync_update = StateSyncUpdate {
            block_header: response.block_header,
            note_updates,
            transactions_to_commit,
            new_mmr_peaks: new_peaks,
            new_authentication_nodes,
            updated_accounts: AccountUpdates::new(
                updated_public_accounts,
                mismatched_private_accounts,
            ),
            block_has_relevant_notes: incoming_block_has_relevant_notes,
            transactions_to_discard,
            tags_to_remove,
        };

        // Apply received and computed updates to the store
        self.store
            .apply_state_sync(state_sync_update)
            .await
            .map_err(ClientError::StoreError)?;

        Ok(sync_summary)
    }

        Ok(sync_summary)
    }
}

pub(crate) fn get_nullifier_prefix(nullifier: &Nullifier) -> u16 {
    (nullifier.inner()[3].as_int() >> FILTER_ID_SHIFT) as u16
}

// SYNC SUMMARY
// ================================================================================================

/// Contains stats about the sync operation.
pub struct SyncSummary {
    /// Block number up to which the client has been synced.
    pub block_num: BlockNumber,
    /// IDs of notes that have been committed.
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
        block_num: BlockNumber,
        committed_notes: Vec<NoteId>,
        consumed_notes: Vec<NoteId>,
        updated_accounts: Vec<AccountId>,
        locked_accounts: Vec<AccountId>,
        committed_transactions: Vec<TransactionId>,
    ) -> Self {
        Self {
            block_num,
            committed_notes,
            consumed_notes,
            updated_accounts,
            locked_accounts,
            committed_transactions,
        }
    }

    pub fn new_empty(block_num: BlockNumber) -> Self {
        Self {
            block_num,
            committed_notes: vec![],
            consumed_notes: vec![],
            updated_accounts: vec![],
            locked_accounts: vec![],
            committed_transactions: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.committed_notes.is_empty()
            && self.consumed_notes.is_empty()
            && self.updated_accounts.is_empty()
            && self.locked_accounts.is_empty()
    }

    pub fn combine_with(&mut self, mut other: Self) {
        self.block_num = max(self.block_num, other.block_num);
        self.committed_notes.append(&mut other.committed_notes);
        self.consumed_notes.append(&mut other.consumed_notes);
        self.updated_accounts.append(&mut other.updated_accounts);
        self.locked_accounts.append(&mut other.locked_accounts);
    }
}

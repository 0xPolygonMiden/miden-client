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

use crate::{store::NoteFilter, Client, ClientError};

mod block_header;

mod tag;
pub use tag::{NoteTagRecord, NoteTagSource};

mod state_sync;
pub use state_sync::{OnNoteReceived, OnNullifierReceived, StateSync, StateSyncUpdate};

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
        self.store.get_sync_height().await.map_err(|err| err.into())
    }

    /// Syncs the client's state with the current state of the Miden network and returns a
    /// [SyncSummary] corresponding to the local state update.
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
    ///    [crate::note::NoteScreener])
    /// 5. Transactions are updated with their new states.
    /// 6. Tracked public accounts are updated and private accounts are validated against the node
    ///    state.
    /// 7. The MMR is updated with the new peaks and authentication nodes.
    /// 8. All updates are applied to the store to be persisted.
    pub async fn sync_state(&mut self) -> Result<SyncSummary, ClientError> {
        _ = self.ensure_genesis_in_place().await?;

        let state_sync = StateSync::new(
            self.rpc_api.clone(),
            Box::new({
                let store_clone = self.store.clone();
                move |committed_note, public_note| {
                    Box::pin(on_note_received(store_clone.clone(), committed_note, public_note))
                }
            }),
            Box::new(move |_committed_note| Box::pin(async { Ok(true) })),
        );

        // Get current state of the client
        let current_block_num = self.store.get_sync_height().await?;
        let (current_block, has_relevant_notes) = self
            .store
            .get_block_header_by_num(current_block_num)
            .await?
            .expect("Current block should be in the store");

        let accounts = self
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

        // Get the sync update from the network
        let state_sync_update = state_sync
            .sync_state(
                current_block,
                has_relevant_notes,
                self.build_current_partial_mmr(false).await?,
                accounts,
                note_tags,
                unspent_input_notes,
                unspent_output_notes,
            )
            .await?;

        let sync_summary: SyncSummary = (&state_sync_update).into();

        // Apply received and computed updates to the store
        self.store
            .apply_state_sync(state_sync_update)
            .await
            .map_err(ClientError::StoreError)?;

        self.update_mmr_data().await?;

        Ok(sync_summary)
    }
}

pub(crate) fn get_nullifier_prefix(nullifier: &Nullifier) -> u16 {
    (nullifier.inner()[3].as_int() >> FILTER_ID_SHIFT) as u16
}

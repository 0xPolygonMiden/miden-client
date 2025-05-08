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
//! # async fn run_sync(client: &mut Client) -> Result<(), ClientError> {
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

use alloc::{boxed::Box, collections::BTreeSet, vec::Vec};
use core::cmp::max;

pub(crate) use block_header::MAX_BLOCK_NUMBER_DELTA;
use miden_objects::{
    account::AccountId,
    block::BlockNumber,
    note::{NoteId, NoteTag},
    transaction::{PartialBlockchain, TransactionId},
};
use miden_tx::utils::{Deserializable, DeserializationError, Serializable};

use crate::{
    Client, ClientError,
    store::{NoteFilter, TransactionFilter},
};
mod block_header;

mod tag;
pub use tag::{NoteTagRecord, NoteTagSource};

mod state_sync;
pub use state_sync::{OnNoteReceived, StateSync, on_note_received};

mod state_sync_update;
pub use state_sync_update::{AccountUpdates, BlockUpdates, StateSyncUpdate, TransactionUpdates};

/// Client synchronization methods.
impl Client {
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

        let state_sync = StateSync::new(
            self.rpc_api.clone(),
            Box::new({
                let store_clone = self.store.clone();
                move |committed_note, public_note| {
                    Box::pin(on_note_received(store_clone.clone(), committed_note, public_note))
                }
            }),
            self.tx_graceful_blocks,
        );

        // Get current state of the client
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

        let uncommitted_transactions =
            self.store.get_transactions(TransactionFilter::Uncommitted).await?;

        // Build current partial MMR
        let current_partial_mmr = self.build_current_partial_mmr().await?;

        let all_block_numbers = (0..current_partial_mmr.forest())
            .filter_map(|block_num| {
                current_partial_mmr.is_tracked(block_num).then_some(BlockNumber::from(
                    u32::try_from(block_num).expect("block number should be less than u32::MAX"),
                ))
            })
            .collect::<BTreeSet<_>>();

        let block_headers = self
            .store
            .get_block_headers(&all_block_numbers)
            .await?
            .into_iter()
            .map(|(header, _has_notes)| header);

        // Get the sync update from the network
        let state_sync_update = state_sync
            .sync_state(
                PartialBlockchain::new(current_partial_mmr, block_headers)?,
                accounts,
                note_tags,
                unspent_input_notes,
                unspent_output_notes,
                uncommitted_transactions,
            )
            .await?;

        let sync_summary: SyncSummary = (&state_sync_update).into();

        // Apply received and computed updates to the store
        self.store
            .apply_state_sync(state_sync_update)
            .await
            .map_err(ClientError::StoreError)?;

        // Remove irrelevant block headers
        self.store.prune_irrelevant_blocks().await?;

        Ok(sync_summary)
    }
}

// SYNC SUMMARY
// ================================================================================================

/// Contains stats about the sync operation.
#[derive(Debug, PartialEq)]
pub struct SyncSummary {
    /// Block number up to which the client has been synced.
    pub block_num: BlockNumber,
    /// IDs of new public notes that the client has received.
    pub new_public_notes: Vec<NoteId>,
    /// IDs of tracked notes that have been committed.
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
        new_public_notes: Vec<NoteId>,
        committed_notes: Vec<NoteId>,
        consumed_notes: Vec<NoteId>,
        updated_accounts: Vec<AccountId>,
        locked_accounts: Vec<AccountId>,
        committed_transactions: Vec<TransactionId>,
    ) -> Self {
        Self {
            block_num,
            new_public_notes,
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
            new_public_notes: vec![],
            committed_notes: vec![],
            consumed_notes: vec![],
            updated_accounts: vec![],
            locked_accounts: vec![],
            committed_transactions: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.new_public_notes.is_empty()
            && self.committed_notes.is_empty()
            && self.consumed_notes.is_empty()
            && self.updated_accounts.is_empty()
            && self.locked_accounts.is_empty()
            && self.committed_transactions.is_empty()
    }

    pub fn combine_with(&mut self, mut other: Self) {
        self.block_num = max(self.block_num, other.block_num);
        self.new_public_notes.append(&mut other.new_public_notes);
        self.committed_notes.append(&mut other.committed_notes);
        self.consumed_notes.append(&mut other.consumed_notes);
        self.updated_accounts.append(&mut other.updated_accounts);
        self.locked_accounts.append(&mut other.locked_accounts);
        self.committed_transactions.append(&mut other.committed_transactions);
    }
}

impl Serializable for SyncSummary {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        self.block_num.write_into(target);
        self.new_public_notes.write_into(target);
        self.committed_notes.write_into(target);
        self.consumed_notes.write_into(target);
        self.updated_accounts.write_into(target);
        self.locked_accounts.write_into(target);
        self.committed_transactions.write_into(target);
    }
}

impl Deserializable for SyncSummary {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, DeserializationError> {
        let block_num = BlockNumber::read_from(source)?;
        let new_public_notes = Vec::<NoteId>::read_from(source)?;
        let committed_notes = Vec::<NoteId>::read_from(source)?;
        let consumed_notes = Vec::<NoteId>::read_from(source)?;
        let updated_accounts = Vec::<AccountId>::read_from(source)?;
        let locked_accounts = Vec::<AccountId>::read_from(source)?;
        let committed_transactions = Vec::<TransactionId>::read_from(source)?;

        Ok(Self {
            block_num,
            new_public_notes,
            committed_notes,
            consumed_notes,
            updated_accounts,
            locked_accounts,
            committed_transactions,
        })
    }
}

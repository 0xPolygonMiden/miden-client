//! Provides the client APIs for synchronizing the client's local state with the Miden
//! rollup network. It ensures that the client maintains a valid, up-to-date view of the chain.

use alloc::vec::Vec;
use core::cmp::max;
use std::{boxed::Box, collections::BTreeMap};

use miden_objects::{
    accounts::{AccountHeader, AccountId},
    crypto::{
        merkle::{InOrderIndex, MmrPeaks},
        rand::FeltRng,
    },
    notes::{NoteId, NoteInclusionProof, Nullifier},
    transaction::TransactionId,
    BlockHeader, Digest,
};
use state_sync::fetch_public_note_details;

use crate::{
    accounts::AccountUpdates,
    notes::NoteUpdates,
    rpc::domain::{
        notes::CommittedNote, nullifiers::NullifierUpdate, transactions::TransactionUpdate,
    },
    store::{InputNoteRecord, NoteFilter, OutputNoteRecord},
    transactions::TransactionUpdates,
    Client, ClientError,
};

mod block_headers;

mod tags;
pub use tags::{NoteTagRecord, NoteTagSource};

mod state_sync;
pub use state_sync::{StateSync, SyncStatus};

/// Contains all information needed to apply the update in the store after syncing with the node.
pub struct StateSyncUpdate {
    /// The new block header, returned as part of the
    /// [StateSyncInfo](crate::rpc::domain::sync::StateSyncInfo)
    pub block_header: BlockHeader,
    /// Information about note changes after the sync.
    pub note_updates: NoteUpdates,
    /// Information about transaction changes after the sync.
    pub transaction_updates: TransactionUpdates,
    /// New MMR peaks for the locally tracked MMR of the blockchain.
    pub new_mmr_peaks: MmrPeaks,
    /// New authentications nodes that are meant to be stored in order to authenticate block
    /// headers.
    pub new_authentication_nodes: Vec<(InOrderIndex, Digest)>,
    /// Information abount account changes after the sync.
    pub account_updates: AccountUpdates,
    /// Tag records that are no longer relevant.
    pub tags_to_remove: Vec<NoteTagRecord>,
}

impl From<&StateSyncUpdate> for SyncSummary {
    fn from(value: &StateSyncUpdate) -> Self {
        SyncSummary::new(
            value.block_header.block_num(),
            value.note_updates.new_input_notes().iter().map(|n| n.id()).collect(),
            value.note_updates.committed_note_ids().into_iter().collect(),
            value.note_updates.consumed_note_ids().into_iter().collect(),
            value
                .account_updates
                .updated_public_accounts()
                .iter()
                .map(|acc| acc.id())
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

        let current_block_num = self.store.get_sync_height().await?;
        let mut total_sync_summary = SyncSummary::new_empty(current_block_num);

        loop {
            // Get current state of the client
            let current_block_num = self.store.get_sync_height().await?;
            let current_block = self.store.get_block_header_by_num(current_block_num).await?.0;

            let accounts: Vec<AccountHeader> = self
                .store
                .get_account_headers()
                .await?
                .into_iter()
                .map(|(acc_header, _)| acc_header)
                .collect();

            // Get the sync update from the network
            let rpc_clone = self.rpc_api.clone();
            let status = StateSync::new(
                self.rpc_api.clone(),
                current_block,
                accounts,
                self.store.get_unique_note_tags().await?.into_iter().collect(),
                self.store.get_expected_note_ids().await?,
                Box::new(move |note, block_header| {
                    Box::pin(fetch_public_note_details(
                        rpc_clone.clone(),
                        *note.note_id(),
                        block_header,
                    ))
                }),
                self.store.get_unspent_input_note_nullifiers().await?,
            )
            .sync_state_step()
            .await?;

            let (is_last_block, relevant_sync_info) = if let Some(status) = status {
                (
                    matches!(status, SyncStatus::SyncedToLastBlock(_)),
                    status.into_relevant_sync_info(),
                )
            } else {
                break;
            };

            let (note_updates, transaction_updates, tags_to_remove) = self
                .note_state_update(
                    relevant_sync_info.new_notes,
                    &relevant_sync_info.block_header,
                    relevant_sync_info.expected_note_inclusions,
                    relevant_sync_info.nullifiers,
                    relevant_sync_info.committed_transactions,
                )
                .await?;

            let (new_mmr_peaks, new_authentication_nodes) =
                self.apply_mmr_changes(relevant_sync_info.mmr_delta).await?;

            let state_sync_update = StateSyncUpdate {
                block_header: relevant_sync_info.block_header,
                note_updates,
                transaction_updates,
                new_mmr_peaks,
                new_authentication_nodes,
                account_updates: AccountUpdates::new(
                    relevant_sync_info.updated_public_accounts,
                    relevant_sync_info.mismatched_private_accounts,
                ),
                tags_to_remove,
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

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Returns the [NoteUpdates] containing new public note and committed input/output notes and a
    /// list or note tag records to be removed from the store.
    async fn committed_note_updates(
        &self,
        expected_note_inclusions: Vec<CommittedNote>,
        block_header: &BlockHeader,
    ) -> Result<(NoteUpdates, Vec<NoteTagRecord>), ClientError> {
        let relevant_note_filter = NoteFilter::List(
            expected_note_inclusions.iter().map(|note| note.note_id()).cloned().collect(),
        );

        let mut committed_input_notes: BTreeMap<NoteId, InputNoteRecord> = self
            .store
            .get_input_notes(relevant_note_filter.clone())
            .await?
            .into_iter()
            .map(|n| (n.id(), n))
            .collect();

        let mut committed_output_notes: BTreeMap<NoteId, OutputNoteRecord> = self
            .store
            .get_output_notes(relevant_note_filter)
            .await?
            .into_iter()
            .map(|n| (n.id(), n))
            .collect();

        let mut committed_tracked_input_notes = vec![];
        let mut committed_tracked_output_notes = vec![];
        let mut removed_tags = vec![];

        for committed_note in expected_note_inclusions {
            let inclusion_proof = NoteInclusionProof::new(
                block_header.block_num(),
                committed_note.note_index(),
                committed_note.merkle_path().clone(),
            )?;

            if let Some(mut note_record) = committed_input_notes.remove(committed_note.note_id()) {
                // The note belongs to our locally tracked set of input notes

                let inclusion_proof_received = note_record
                    .inclusion_proof_received(inclusion_proof.clone(), committed_note.metadata())?;
                let block_header_received = note_record.block_header_received(*block_header)?;

                removed_tags.push((&note_record).try_into()?);

                if inclusion_proof_received || block_header_received {
                    committed_tracked_input_notes.push(note_record);
                }
            }

            if let Some(mut note_record) = committed_output_notes.remove(committed_note.note_id()) {
                // The note belongs to our locally tracked set of output notes

                if note_record.inclusion_proof_received(inclusion_proof.clone())? {
                    committed_tracked_output_notes.push(note_record);
                }
            }
        }

        Ok((
            NoteUpdates::new(
                vec![],
                vec![],
                committed_tracked_input_notes,
                committed_tracked_output_notes,
            ),
            removed_tags,
        ))
    }

    /// Returns the [NoteUpdates] containing consumed input/output notes and a list of IDs of the
    /// transactions that were discarded.
    async fn consumed_note_updates(
        &self,
        nullifiers: Vec<NullifierUpdate>,
        committed_transactions: &[TransactionUpdate],
    ) -> Result<(NoteUpdates, Vec<TransactionId>), ClientError> {
        let nullifier_filter = NoteFilter::Nullifiers(
            nullifiers.iter().map(|nullifier_update| nullifier_update.nullifier).collect(),
        );

        let mut consumed_input_notes: BTreeMap<Nullifier, InputNoteRecord> = self
            .store
            .get_input_notes(nullifier_filter.clone())
            .await?
            .into_iter()
            .map(|n| (n.nullifier(), n))
            .collect();

        let mut consumed_output_notes: BTreeMap<Nullifier, OutputNoteRecord> = self
            .store
            .get_output_notes(nullifier_filter)
            .await?
            .into_iter()
            .map(|n| {
                (
                    n.nullifier()
                        .expect("Output notes returned by this query should have nullifiers"),
                    n,
                )
            })
            .collect();

        let mut consumed_tracked_input_notes = vec![];
        let mut consumed_tracked_output_notes = vec![];

        // Committed transactions
        for transaction_update in committed_transactions {
            let transaction_nullifiers: Vec<Nullifier> = consumed_input_notes
                .iter()
                .filter_map(|(nullifier, note_record)| {
                    if note_record.is_processing()
                        && note_record.consumer_transaction_id()
                            == Some(&transaction_update.transaction_id)
                    {
                        Some(nullifier)
                    } else {
                        None
                    }
                })
                .cloned()
                .collect();

            for nullifier in transaction_nullifiers {
                if let Some(mut input_note_record) = consumed_input_notes.remove(&nullifier) {
                    if input_note_record.transaction_committed(
                        transaction_update.transaction_id,
                        transaction_update.block_num,
                    )? {
                        consumed_tracked_input_notes.push(input_note_record);
                    }
                }
            }
        }

        // Nullified notes
        let mut discarded_transactions = vec![];
        for nullifier_update in nullifiers {
            let nullifier = nullifier_update.nullifier;
            let block_num = nullifier_update.block_num;

            if let Some(mut input_note_record) = consumed_input_notes.remove(&nullifier) {
                if input_note_record.is_processing() {
                    discarded_transactions.push(
                        *input_note_record
                            .consumer_transaction_id()
                            .expect("Processing note should have consumer transaction id"),
                    );
                }

                if input_note_record.consumed_externally(nullifier, block_num)? {
                    consumed_tracked_input_notes.push(input_note_record);
                }
            }

            if let Some(mut output_note_record) = consumed_output_notes.remove(&nullifier) {
                if output_note_record.nullifier_received(nullifier, block_num)? {
                    consumed_tracked_output_notes.push(output_note_record);
                }
            }
        }

        Ok((
            NoteUpdates::new(
                vec![],
                vec![],
                consumed_tracked_input_notes,
                consumed_tracked_output_notes,
            ),
            discarded_transactions,
        ))
    }

    /// Compares the state of tracked notes with the updates received from the node and returns the
    /// note and transaction changes that should be applied to the store plus a list of note tag
    /// records that are no longer relevant.
    ///
    /// The note changes might include:
    /// * New notes that we received from the node and might be relevant to the client.
    /// * Tracked expected notes that were committed in the block.
    /// * Tracked notes that were being processed by a transaction that got committed.
    /// * Tracked notes that were nullified by an external transaction.
    ///
    /// The transaction changes might include:
    /// * Transactions that were committed in the block. Some of these might me tracked by the
    ///   client and need to be marked as committed.
    /// * Local tracked transactions that were discarded because the notes that they were processing
    ///   were nullified by an another transaction.
    async fn note_state_update(
        &self,
        new_input_notes: Vec<InputNoteRecord>,
        block_header: &BlockHeader,
        committed_notes: Vec<CommittedNote>,
        nullifiers: Vec<NullifierUpdate>,
        committed_transactions: Vec<TransactionUpdate>,
    ) -> Result<(NoteUpdates, TransactionUpdates, Vec<NoteTagRecord>), ClientError> {
        let (committed_note_updates, tags_to_remove) =
            self.committed_note_updates(committed_notes, block_header).await?;

        let (consumed_note_updates, discarded_transactions) =
            self.consumed_note_updates(nullifiers, &committed_transactions).await?;

        let note_updates = NoteUpdates::new(new_input_notes, vec![], vec![], vec![])
            .combine_with(committed_note_updates)
            .combine_with(consumed_note_updates);

        let transaction_updates =
            TransactionUpdates::new(committed_transactions, discarded_transactions);

        Ok((note_updates, transaction_updates, tags_to_remove))
    }
}

pub(crate) fn get_nullifier_prefix(nullifier: &Nullifier) -> u16 {
    (nullifier.inner()[3].as_int() >> FILTER_ID_SHIFT) as u16
}

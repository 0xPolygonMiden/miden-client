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

use alloc::{collections::BTreeMap, vec::Vec};
use core::cmp::max;

use miden_objects::{
    Digest,
    account::{Account, AccountHeader, AccountId},
    block::{BlockHeader, BlockNumber},
    note::{NoteId, NoteInclusionProof, NoteTag, Nullifier},
    transaction::TransactionId,
};
use miden_tx::utils::{Deserializable, DeserializationError, Serializable};
use tracing::info;

use crate::{
    Client, ClientError,
    note::NoteUpdates,
    rpc::domain::{
        note::CommittedNote, nullifier::NullifierUpdate, transaction::TransactionUpdate,
    },
    store::{AccountUpdates, InputNoteRecord, NoteFilter, OutputNoteRecord, TransactionFilter},
    transaction::{TransactionRecord, TransactionStatus, TransactionUpdates},
};

mod block_header;
use block_header::apply_mmr_changes;

mod tag;
pub use tag::{NoteTagRecord, NoteTagSource};

/// The number of blocks that are considered old enough to discard pending transactions.
pub const TX_GRACEFUL_BLOCKS: u32 = 20;
mod state_sync_update;
pub use state_sync_update::StateSyncUpdate;

/// Contains stats about the sync operation.
#[derive(Debug, PartialEq)]
pub struct SyncSummary {
    /// Block number up to which the client has been synced.
    pub block_num: BlockNumber,
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
            && self.committed_transactions.is_empty()
    }

    pub fn combine_with(&mut self, mut other: Self) {
        self.block_num = max(self.block_num, other.block_num);
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
        let committed_notes = Vec::<NoteId>::read_from(source)?;
        let consumed_notes = Vec::<NoteId>::read_from(source)?;
        let updated_accounts = Vec::<AccountId>::read_from(source)?;
        let locked_accounts = Vec::<AccountId>::read_from(source)?;
        let committed_transactions = Vec::<TransactionId>::read_from(source)?;

        Ok(Self {
            block_num,
            committed_notes,
            consumed_notes,
            updated_accounts,
            locked_accounts,
            committed_transactions,
        })
    }
}

enum SyncStatus {
    SyncedToLastBlock(SyncSummary),
    SyncedToBlock(SyncSummary),
}

impl SyncStatus {
    pub fn into_sync_summary(self) -> SyncSummary {
        match self {
            SyncStatus::SyncedToBlock(summary) | SyncStatus::SyncedToLastBlock(summary) => summary,
        }
    }
}

// CONSTANTS
// ================================================================================================

/// Client syncronization methods.
impl Client {
    // SYNC STATE
    // --------------------------------------------------------------------------------------------

    /// Returns the block number of the last state sync block.
    pub async fn get_sync_height(&self) -> Result<BlockNumber, ClientError> {
        self.store.get_sync_height().await.map_err(Into::into)
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
    ///    [`crate::note::NoteScreener`])
    /// 5. Transactions are updated with their new states.
    /// 6. Tracked public accounts are updated and off-chain accounts are validated against the node
    ///    state.
    /// 7. The MMR is updated with the new peaks and authentication nodes.
    /// 8. All updates are applied to the store to be persisted.
    pub async fn sync_state(&mut self) -> Result<SyncSummary, ClientError> {
        let starting_block_num = self.get_sync_height().await?;

        _ = self.ensure_genesis_in_place().await?;
        let mut total_sync_summary = SyncSummary::new_empty(0.into());
        loop {
            let response = self.sync_state_once().await?;
            let is_last_block = matches!(response, SyncStatus::SyncedToLastBlock(_));
            total_sync_summary.combine_with(response.into_sync_summary());

            if is_last_block {
                break;
            }
        }
        self.update_mmr_data().await?;
        // Sync and apply nullifiers
        total_sync_summary.combine_with(self.sync_nullifiers(starting_block_num).await?);

        Ok(total_sync_summary)
    }

    async fn sync_state_once(&mut self) -> Result<SyncStatus, ClientError> {
        let current_block_num = self.store.get_sync_height().await?;

        let accounts: Vec<AccountHeader> = self
            .store
            .get_account_headers()
            .await?
            .into_iter()
            .map(|(acc_header, _)| acc_header)
            .collect();

        let note_tags: Vec<NoteTag> =
            self.store.get_unique_note_tags().await?.into_iter().collect();

        // Send request
        let account_ids: Vec<AccountId> = accounts.iter().map(AccountHeader::id).collect();
        let response = self.rpc_api.sync_state(current_block_num, &account_ids, &note_tags).await?;

        // We don't need to continue if the chain has not advanced, there are no new changes
        if response.block_header.block_num() == current_block_num {
            return Ok(SyncStatus::SyncedToLastBlock(SyncSummary::new_empty(current_block_num)));
        }

        let (note_updates, tags_to_remove) = self
            .committed_note_updates(response.note_inclusions, &response.block_header)
            .await?;

        let incoming_block_has_relevant_notes = self.check_block_relevance(&note_updates).await?;

        let transactions_to_commit = self.get_transactions_to_commit(response.transactions).await?;

        let (public_accounts, private_accounts): (Vec<_>, Vec<_>) =
            accounts.into_iter().partition(|account_header| account_header.id().is_public());

        let updated_public_accounts = self
            .get_updated_public_accounts(&response.account_commitment_updates, &public_accounts)
            .await?;

        let mismatched_private_accounts = self
            .validate_local_account_commitments(
                &response.account_commitment_updates,
                &private_accounts,
            )
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
            note_updates.committed_note_ids().into_iter().collect(),
            note_updates.consumed_note_ids().into_iter().collect(),
            updated_public_accounts.iter().map(Account::id).collect(),
            mismatched_private_accounts.iter().map(|(acc_id, _)| *acc_id).collect(),
            transactions_to_commit.iter().map(|tx| tx.transaction_id).collect(),
        );
        let response_block_num = response.block_header.block_num();

        let transactions_to_discard = vec![];

        // Find old pending transactions before starting the database transaction
        let graceful_block_num =
            response_block_num.checked_sub(TX_GRACEFUL_BLOCKS).unwrap_or_default();
        // Retain old pending transactions
        let mut stale_transactions: Vec<TransactionRecord> = self
            .store
            .get_transactions(TransactionFilter::ExpiredBefore(graceful_block_num))
            .await?;

        stale_transactions.retain(|tx| {
            !transactions_to_commit
                .iter()
                .map(|tx| tx.transaction_id)
                .collect::<Vec<_>>()
                .contains(&tx.id)
                && !transactions_to_discard.contains(&tx.id)
        });

        let state_sync_update = StateSyncUpdate {
            block_header: response.block_header,
            block_has_relevant_notes: incoming_block_has_relevant_notes,
            new_mmr_peaks: new_peaks,
            new_authentication_nodes,
            note_updates,
            transaction_updates: TransactionUpdates::new(
                transactions_to_commit,
                transactions_to_discard,
                stale_transactions,
            ),
            account_updates: AccountUpdates::new(
                updated_public_accounts,
                mismatched_private_accounts,
            ),
            tags_to_remove,
        };

        // Apply received and computed updates to the store
        self.store
            .apply_state_sync(state_sync_update)
            .await
            .map_err(ClientError::StoreError)?;

        if response.chain_tip == response_block_num {
            Ok(SyncStatus::SyncedToLastBlock(sync_summary))
        } else {
            Ok(SyncStatus::SyncedToBlock(sync_summary))
        }
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    async fn sync_nullifiers(
        &mut self,
        starting_block_num: BlockNumber,
    ) -> Result<SyncSummary, ClientError> {
        // To receive information about added nullifiers, we reduce them to the higher 16 bits
        // Note that besides filtering by nullifier prefixes, the node also filters by block number
        // (it only returns nullifiers from current_block_num until
        // response.block_header.block_num())
        let nullifiers_tags: Vec<u16> = self
            .store
            .get_unspent_input_note_nullifiers()
            .await?
            .iter()
            .map(Nullifier::prefix)
            .collect();

        let mut nullifiers = self
            .rpc_api
            .check_nullifiers_by_prefix(&nullifiers_tags, starting_block_num)
            .await?;

        // Discard nullifiers that are newer than the current block (this might happen if the block
        // changes between the sync_state and the check_nullifier calls)
        let current_block_num = self.get_sync_height().await?;
        nullifiers.retain(|update| update.block_num <= current_block_num.as_u32());

        // Committed transactions
        let committed_transactions = self
            .store
            .get_transactions(TransactionFilter::All)
            .await?
            .into_iter()
            .filter_map(|tx| {
                if let TransactionStatus::Committed(block_num) = tx.transaction_status {
                    Some(TransactionUpdate {
                        transaction_id: tx.id,
                        account_id: tx.account_id,
                        block_num: block_num.as_u32(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let (consumed_note_updates, transactions_to_discard) =
            self.consumed_note_updates(&nullifiers, &committed_transactions).await?;

        // Store summary to return later
        let sync_summary = SyncSummary::new(
            0.into(),
            consumed_note_updates.committed_note_ids().into_iter().collect(),
            consumed_note_updates.consumed_note_ids().into_iter().collect(),
            vec![],
            vec![],
            committed_transactions.iter().map(|tx| tx.transaction_id).collect(),
        );

        // Apply received and computed updates to the store
        self.store
            .apply_nullifiers(consumed_note_updates, transactions_to_discard)
            .await
            .map_err(ClientError::StoreError)?;

        Ok(sync_summary)
    }

    /// Returns the [`NoteUpdates`] containing new public note and committed input/output notes and
    /// a list or note tag records to be removed from the store.
    async fn committed_note_updates(
        &mut self,
        committed_notes: Vec<CommittedNote>,
        block_header: &BlockHeader,
    ) -> Result<(NoteUpdates, Vec<NoteTagRecord>), ClientError> {
        // We'll only pick committed notes that we are tracking as input/output notes. Since the
        // sync response contains notes matching either the provided accounts or the provided tag
        // we might get many notes when we only care about a few of those.
        let relevant_note_filter =
            NoteFilter::List(committed_notes.iter().map(CommittedNote::note_id).copied().collect());

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

        let mut new_public_notes = vec![];
        let mut committed_tracked_input_notes = vec![];
        let mut committed_tracked_output_notes = vec![];
        let mut removed_tags = vec![];

        for committed_note in committed_notes {
            let inclusion_proof = NoteInclusionProof::new(
                block_header.block_num(),
                committed_note.note_index(),
                committed_note.merkle_path().clone(),
            )?;

            if let Some(mut note_record) = committed_input_notes.remove(committed_note.note_id()) {
                // The note belongs to our locally tracked set of input notes

                let inclusion_proof_received = note_record
                    .inclusion_proof_received(inclusion_proof.clone(), committed_note.metadata())?;
                let block_header_received = note_record.block_header_received(block_header)?;

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

            if !committed_input_notes.contains_key(committed_note.note_id())
                && !committed_output_notes.contains_key(committed_note.note_id())
            {
                // The note is public and we are not tracking it, push to the list of IDs to query
                new_public_notes.push(*committed_note.note_id());
            }
        }

        // Query the node for input note data and build the entities
        let new_public_notes =
            self.fetch_public_note_details(&new_public_notes, block_header).await?;

        Ok((
            NoteUpdates::new(
                [new_public_notes, committed_tracked_input_notes].concat(),
                committed_tracked_output_notes,
            ),
            removed_tags,
        ))
    }

    /// Returns the [`NoteUpdates`] containing consumed input/output notes and a list of IDs of the
    /// transactions that were discarded.
    async fn consumed_note_updates(
        &mut self,
        nullifiers: &[NullifierUpdate],
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
                .copied()
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
            NoteUpdates::new(consumed_tracked_input_notes, consumed_tracked_output_notes),
            discarded_transactions,
        ))
    }

    /// Queries the node for all received notes that aren't being locally tracked in the client.
    ///
    /// The client can receive metadata for private notes that it's not tracking. In this case,
    /// notes are ignored for now as they become useless until details are imported.
    async fn fetch_public_note_details(
        &mut self,
        query_notes: &[NoteId],
        block_header: &BlockHeader,
    ) -> Result<Vec<InputNoteRecord>, ClientError> {
        if query_notes.is_empty() {
            return Ok(vec![]);
        }
        info!("Getting note details for notes that are not being tracked.");

        let mut return_notes = self
            .rpc_api
            .get_public_note_records(query_notes, self.store.get_current_timestamp())
            .await?;

        for note in &mut return_notes {
            note.block_header_received(block_header)?;
        }

        Ok(return_notes)
    }

    /// Extracts information about transactions for uncommitted transactions that the client is
    /// tracking from the received [`SyncStateResponse`].
    async fn get_transactions_to_commit(
        &self,
        mut transactions: Vec<TransactionUpdate>,
    ) -> Result<Vec<TransactionUpdate>, ClientError> {
        // Get current uncommitted transactions
        let uncommitted_transaction_ids = self
            .store
            .get_transactions(TransactionFilter::Uncomitted)
            .await?
            .into_iter()
            .map(|tx| tx.id)
            .collect::<Vec<_>>();

        transactions.retain(|transaction_update| {
            uncommitted_transaction_ids.contains(&transaction_update.transaction_id)
        });

        Ok(transactions)
    }

    async fn get_updated_public_accounts(
        &mut self,
        account_updates: &[(AccountId, Digest)],
        current_public_accounts: &[AccountHeader],
    ) -> Result<Vec<Account>, ClientError> {
        let mut mismatched_public_accounts = vec![];

        for (id, commitment) in account_updates {
            // check if this updated account is tracked by the client
            if let Some(account) = current_public_accounts
                .iter()
                .find(|acc| *id == acc.id() && *commitment != acc.commitment())
            {
                mismatched_public_accounts.push(account);
            }
        }

        self.rpc_api
            .get_updated_public_accounts(&mismatched_public_accounts)
            .await
            .map_err(ClientError::RpcError)
    }

    /// Validates account commitment updates and returns a vector with all the private account
    /// mismatches.
    ///
    /// Private account mismatches happen when the commitment account of the local tracked account
    /// doesn't match the commitment account of the account in the node. This would be an anomaly
    /// and may happen for two main reasons:
    /// - A different client made a transaction with the account, changing its state.
    /// - The local transaction that modified the local state didn't go through, rendering the local
    ///   account state outdated.
    async fn validate_local_account_commitments(
        &mut self,
        account_updates: &[(AccountId, Digest)],
        current_private_accounts: &[AccountHeader],
    ) -> Result<Vec<(AccountId, Digest)>, ClientError> {
        let mut mismatched_accounts = vec![];

        for (remote_account_id, remote_account_commitment) in account_updates {
            // ensure that if we track that account, it has the same commitment
            let mismatched_account = current_private_accounts.iter().find(|acc| {
                *remote_account_id == acc.id() && *remote_account_commitment != acc.commitment()
            });

            // Private accounts should always have the latest known state. If we receive a stale
            // update we ignore it.
            if mismatched_account.is_some() {
                let account_by_commitment =
                    self.store.get_account_header_by_commitment(*remote_account_commitment).await?;

                if account_by_commitment.is_none() {
                    mismatched_accounts.push((*remote_account_id, *remote_account_commitment));
                }
            }
        }
        Ok(mismatched_accounts)
    }
}

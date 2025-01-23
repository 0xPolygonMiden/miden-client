//! Provides the client APIs for synchronizing the client's local state with the Miden
//! rollup network. It ensures that the client maintains a valid, up-to-date view of the chain.

use alloc::{collections::BTreeMap, vec::Vec};
use core::cmp::max;

use crypto::merkle::{InOrderIndex, MmrPeaks};
use miden_objects::{
    account::{Account, AccountHeader, AccountId},
    block::{BlockHeader, BlockNumber},
    crypto::{self, rand::FeltRng},
    note::{NoteId, NoteInclusionProof, NoteTag, Nullifier},
    transaction::TransactionId,
    Digest,
};
use tracing::info;

use crate::{
    account::AccountUpdates,
    note::NoteUpdates,
    rpc::domain::{
        note::CommittedNote, nullifier::NullifierUpdate, transaction::TransactionUpdate,
    },
    store::{InputNoteRecord, NoteFilter, OutputNoteRecord, TransactionFilter},
    Client, ClientError,
};

mod block_header;
use block_header::apply_mmr_changes;

mod tag;
pub use tag::{NoteTagRecord, NoteTagSource};

/// Contains stats about the sync operation.
pub struct SyncSummary {
    /// Block number up to which the client has been synced.
    pub block_num: BlockNumber,
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

enum SyncStatus {
    SyncedToLastBlock(SyncSummary),
    SyncedToBlock(SyncSummary),
}

impl SyncStatus {
    pub fn into_sync_summary(self) -> SyncSummary {
        match self {
            SyncStatus::SyncedToLastBlock(summary) => summary,
            SyncStatus::SyncedToBlock(summary) => summary,
        }
    }
}

/// Contains all information needed to apply the update in the store after syncing with the node.
pub struct StateSyncUpdate {
    /// The new block header, returned as part of the
    /// [StateSyncInfo](crate::rpc::domain::sync::StateSyncInfo)
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
    ///    [crate::note::NoteScreener])
    /// 5. Transactions are updated with their new states.
    /// 6. Tracked public accounts are updated and off-chain accounts are validated against the node
    ///    state.
    /// 7. The MMR is updated with the new peaks and authentication nodes.
    /// 8. All updates are applied to the store to be persisted.
    pub async fn sync_state(&mut self) -> Result<SyncSummary, ClientError> {
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

        // To receive information about added nullifiers, we reduce them to the higher 16 bits
        // Note that besides filtering by nullifier prefixes, the node also filters by block number
        // (it only returns nullifiers from current_block_num until
        // response.block_header.block_num())
        let nullifiers_tags: Vec<u16> = self
            .store
            .get_unspent_input_note_nullifiers()
            .await?
            .iter()
            .map(get_nullifier_prefix)
            .collect();

        // Send request
        let account_ids: Vec<AccountId> = accounts.iter().map(|acc| acc.id()).collect();
        let response = self
            .rpc_api
            .sync_state(current_block_num, &account_ids, &note_tags, &nullifiers_tags)
            .await?;

        // We don't need to continue if the chain has not advanced, there are no new changes
        if response.block_header.block_num() == current_block_num {
            return Ok(SyncStatus::SyncedToLastBlock(SyncSummary::new_empty(current_block_num)));
        }

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
                current_block,
                has_relevant_notes,
            )?
        };

        // Store summary to return later
        let sync_summary = SyncSummary::new(
            response.block_header.block_num(),
            note_updates.new_input_notes().iter().map(|n| n.id()).collect(),
            note_updates.committed_note_ids().into_iter().collect(),
            note_updates.consumed_note_ids().into_iter().collect(),
            updated_public_accounts.iter().map(|acc| acc.id()).collect(),
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

        if response.chain_tip == response.block_header.block_num() {
            Ok(SyncStatus::SyncedToLastBlock(sync_summary))
        } else {
            Ok(SyncStatus::SyncedToBlock(sync_summary))
        }
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Returns the [NoteUpdates] containing new public note and committed input/output notes and a
    /// list or note tag records to be removed from the store.
    async fn committed_note_updates(
        &mut self,
        committed_notes: Vec<CommittedNote>,
        block_header: &BlockHeader,
    ) -> Result<(NoteUpdates, Vec<NoteTagRecord>), ClientError> {
        // We'll only pick committed notes that we are tracking as input/output notes. Since the
        // sync response contains notes matching either the provided accounts or the provided tag
        // we might get many notes when we only care about a few of those.
        let relevant_note_filter =
            NoteFilter::List(committed_notes.iter().map(|note| note.note_id()).cloned().collect());

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
                new_public_notes,
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
        &mut self,
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

        for note in return_notes.iter_mut() {
            note.block_header_received(*block_header)?;
        }

        Ok(return_notes)
    }

    /// Extracts information about transactions for uncommitted transactions that the client is
    /// tracking from the received [SyncStateResponse].
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

        for (id, hash) in account_updates {
            // check if this updated account is tracked by the client
            if let Some(account) = current_public_accounts
                .iter()
                .find(|acc| *id == acc.id() && *hash != acc.hash())
            {
                mismatched_public_accounts.push(account);
            }
        }

        self.rpc_api
            .get_updated_public_accounts(&mismatched_public_accounts)
            .await
            .map_err(ClientError::RpcError)
    }

    /// Validates account hash updates and returns a vector with all the private account
    /// mismatches.
    ///
    /// Private account mismatches happen when the hash account of the local tracked account
    /// doesn't match the hash account of the account in the node. This would be an anomaly and may
    /// happen for two main reasons:
    /// - A different client made a transaction with the account, changing its state.
    /// - The local transaction that modified the local state didn't go through, rendering the local
    ///   account state outdated.
    async fn validate_local_account_hashes(
        &mut self,
        account_updates: &[(AccountId, Digest)],
        current_private_accounts: &[AccountHeader],
    ) -> Result<Vec<(AccountId, Digest)>, ClientError> {
        let mut mismatched_accounts = vec![];

        for (remote_account_id, remote_account_hash) in account_updates {
            // ensure that if we track that account, it has the same hash
            let mismatched_account = current_private_accounts
                .iter()
                .find(|acc| *remote_account_id == acc.id() && *remote_account_hash != acc.hash());

            // Private accounts should always have the latest known state. If we receive a stale
            // update we ignore it.
            if mismatched_account.is_some() {
                let account_by_hash =
                    self.store.get_account_header_by_hash(*remote_account_hash).await?;

                if account_by_hash.is_none() {
                    mismatched_accounts.push((*remote_account_id, *remote_account_hash));
                }
            }
        }
        Ok(mismatched_accounts)
    }
}

pub(crate) fn get_nullifier_prefix(nullifier: &Nullifier) -> u16 {
    (nullifier.inner()[3].as_int() >> FILTER_ID_SHIFT) as u16
}

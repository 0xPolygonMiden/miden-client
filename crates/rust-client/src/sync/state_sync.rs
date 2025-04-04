use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec::Vec};
use core::{future::Future, pin::Pin};

use miden_objects::{
    Digest,
    account::{Account, AccountHeader, AccountId},
    block::{BlockHeader, BlockNumber},
    crypto::merkle::{InOrderIndex, MmrDelta, MmrPeaks, PartialMmr},
    note::{NoteId, NoteTag},
    transaction::ChainMmr,
};
use tracing::info;

use super::{
    AccountUpdates, BlockUpdates, StateSyncUpdate, TX_GRACEFUL_BLOCKS, TransactionUpdates,
    block_header::fetch_block_header,
};
use crate::{
    ClientError,
    note::{NoteScreener, NoteUpdateTracker},
    rpc::{NodeRpcClient, domain::note::CommittedNote},
    store::{InputNoteRecord, InputNoteState, NoteFilter, OutputNoteRecord, Store, StoreError},
    transaction::TransactionRecord,
};

// SYNC CALLBACKS
// ================================================================================================

/// Callback that gets executed when a new note is received as part of the sync response.
///
/// It receives the committed note received from the network and an optional note record that
/// corresponds to the state of the note in the network (only if the note is public).
///
/// It returns a boolean indicating if the received note update is relevant. If the return value
/// is `false`, it gets discarded. If it is `true`, the update gets committed to the client's store.
pub type OnNoteReceived = Box<
    dyn Fn(
        CommittedNote,
        Option<InputNoteRecord>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, ClientError>>>>,
>;

// STATE SYNC
// ================================================================================================

/// The state sync components encompasses the client's sync logic. It is then used to requset
/// updates from the node and apply them to the relevant elements. The updates are then returned and
/// can be applied to the store to persist the changes.
///
/// When created it receives a callback that will be executed when a new note inclusion is received
/// in the sync response.
pub struct StateSync {
    /// The RPC client used to communicate with the node.
    rpc_api: Arc<dyn NodeRpcClient + Send>,
    /// Callback to be executed when a new note inclusion is received.
    on_note_received: OnNoteReceived,
}

impl StateSync {
    /// Creates a new instance of the state sync component.
    ///
    /// # Arguments
    ///
    /// * `rpc_api` - The RPC client used to communicate with the node.
    /// * `on_note_received` - A callback to be executed when a new note inclusion is received.
    pub fn new(rpc_api: Arc<dyn NodeRpcClient + Send>, on_note_received: OnNoteReceived) -> Self {
        Self { rpc_api, on_note_received }
    }

    /// Syncs the state of the client with the chain tip of the node, returning the updates that
    /// should be applied to the store.
    ///
    /// During the sync process, the client will go through the following steps:
    /// 1. A request is sent to the node to get the state updates. This request includes tracked
    ///    account IDs and the tags of notes that might have changed or that might be of interest to
    ///    the client.
    /// 2. A response is received with the current state of the network. The response includes
    ///    information about new and committed notes, updated accounts, and committed transactions.
    /// 3. Tracked public accounts are updated and private accounts are validated against the node
    ///    state.
    /// 4. Tracked notes are updated with their new states. Notes might be committed or nullified
    ///    during the sync processing.
    /// 5. New notes are checked, and only relevant ones are stored. Relevance is determined by the
    ///    [`OnNoteReceived`] callback.
    /// 6. Transactions are updated with their new states. Transactions might be committed or
    ///    discarded.
    /// 7. The MMR is updated with the new peaks and authentication nodes.
    ///
    /// # Arguments
    /// * `current_chain_mmr` - The current chain MMR.
    /// * `accounts` - All the headers of tracked accounts.
    /// * `note_tags` - The note tags to be used in the sync state request.
    /// * `unspent_input_notes` - The current state of unspent input notes tracked by the client.
    /// * `unspent_output_notes` - The current state of unspent output notes tracked by the client.
    pub async fn sync_state(
        self,
        mut current_chain_mmr: ChainMmr,
        accounts: Vec<AccountHeader>,
        note_tags: Vec<NoteTag>,
        unspent_input_notes: Vec<InputNoteRecord>,
        unspent_output_notes: Vec<OutputNoteRecord>,
        mut uncommitted_transactions: Vec<TransactionRecord>,
    ) -> Result<StateSyncUpdate, ClientError> {
        let block_num = current_chain_mmr.chain_length().checked_sub(1).unwrap_or_default();

        let mut state_sync_update = StateSyncUpdate {
            block_num,
            note_updates: NoteUpdateTracker::new(unspent_input_notes, unspent_output_notes),
            ..Default::default()
        };

        loop {
            if !self
                .sync_state_step(
                    &mut state_sync_update,
                    &mut current_chain_mmr,
                    &accounts,
                    &note_tags,
                )
                .await?
            {
                break;
            }
        }

        self.sync_nullifiers(&mut state_sync_update, block_num).await?;

        // Add stale transactions to the state sync update
        let mut updated_transactions = state_sync_update
            .transaction_updates
            .committed_transactions()
            .iter()
            .map(|tx| tx.transaction_id)
            .chain(state_sync_update.transaction_updates.discarded_transactions().iter().copied());

        let graceful_block_num =
            state_sync_update.block_num.checked_sub(TX_GRACEFUL_BLOCKS).unwrap_or_default();

        uncommitted_transactions.retain(|tx| {
            tx.block_num < graceful_block_num && !updated_transactions.any(|tx_id| tx_id == tx.id)
        });

        state_sync_update.transaction_updates.extend(TransactionUpdates::new(
            vec![],
            vec![],
            uncommitted_transactions,
        ));

        self.update_unverified_notes(&mut state_sync_update, &mut current_chain_mmr)
            .await?;

        Ok(state_sync_update)
    }

    /// Executes a single step of the state sync process, returning `true` if the client should
    /// continue syncing and `false` if the client has reached the chain tip.
    ///
    /// A step in this context means a single request to the node to get the next relevant block and
    /// the changes that happened in it. This block may not be the last one in the chain and
    /// the client may need to call this method multiple times until it reaches the chain tip.
    ///
    /// The `sync_state_update` field of the struct will be updated with the new changes from this
    /// step.
    async fn sync_state_step(
        &self,
        state_sync_update: &mut StateSyncUpdate,
        current_chain_mmr: &mut ChainMmr,
        accounts: &[AccountHeader],
        note_tags: &[NoteTag],
    ) -> Result<bool, ClientError> {
        let account_ids: Vec<AccountId> = accounts.iter().map(AccountHeader::id).collect();

        let response = self
            .rpc_api
            .sync_state(state_sync_update.block_num, &account_ids, note_tags)
            .await?;

        // We don't need to continue if the chain has not advanced, there are no new changes
        if response.block_header.block_num() == state_sync_update.block_num {
            return Ok(false);
        }

        let new_block_num = response.block_header.block_num();
        state_sync_update.block_num = new_block_num;

        let account_updates =
            self.account_state_sync(accounts, &response.account_commitment_updates).await?;

        state_sync_update.account_updates.extend(account_updates);

        // Track the transaction updates for transactions that were committed. Some of these might
        // be tracked by the client and need to be marked as committed.
        state_sync_update.transaction_updates.extend(TransactionUpdates::new(
            response.transactions,
            vec![],
            vec![],
        ));

        let found_relevant_note = self
            .note_state_sync(
                &mut state_sync_update.note_updates,
                response.note_inclusions,
                &response.block_header,
            )
            .await?;

        let (new_mmr_peaks, new_authentication_nodes) = apply_mmr_changes(
            &response.block_header,
            found_relevant_note,
            current_chain_mmr.partial_mmr_mut(),
            response.mmr_delta,
        )?;

        let mut new_blocks = vec![];
        if found_relevant_note || response.chain_tip == new_block_num {
            // Only track relevant blocks or the chain tip
            new_blocks.push((response.block_header, found_relevant_note, new_mmr_peaks));
        }

        state_sync_update
            .block_updates
            .extend(BlockUpdates::new(new_blocks, new_authentication_nodes));

        if response.chain_tip == new_block_num {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Compares the state of tracked accounts with the updates received from the node. The method
    /// updates the `state_sync_update` field with the details of the accounts that need to be
    /// updated.
    ///
    /// The account updates might include:
    /// * Public accounts that have been updated in the node.
    /// * Private accounts that have been marked as mismatched because the current commitment
    ///   doesn't match the one received from the node. The client will need to handle these cases
    ///   as they could be a stale account state or a reason to lock the account.
    async fn account_state_sync(
        &self,
        accounts: &[AccountHeader],
        account_commitment_updates: &[(AccountId, Digest)],
    ) -> Result<AccountUpdates, ClientError> {
        let (public_accounts, private_accounts): (Vec<_>, Vec<_>) =
            accounts.iter().partition(|account_header| account_header.id().is_public());

        let updated_public_accounts = self
            .get_updated_public_accounts(account_commitment_updates, &public_accounts)
            .await?;

        let mismatched_private_accounts = account_commitment_updates
            .iter()
            .filter(|(account_id, digest)| {
                private_accounts
                    .iter()
                    .any(|account| account.id() == *account_id && &account.commitment() != digest)
            })
            .copied()
            .collect::<Vec<_>>();

        Ok(AccountUpdates::new(updated_public_accounts, mismatched_private_accounts))
    }

    /// Queries the node for the latest state of the public accounts that don't match the current
    /// state of the client.
    async fn get_updated_public_accounts(
        &self,
        account_updates: &[(AccountId, Digest)],
        current_public_accounts: &[&AccountHeader],
    ) -> Result<Vec<Account>, ClientError> {
        let mut mismatched_public_accounts = vec![];

        for (id, commitment) in account_updates {
            // check if this updated account state is tracked by the client
            if let Some(account) = current_public_accounts
                .iter()
                .find(|acc| *id == acc.id() && *commitment != acc.commitment())
            {
                mismatched_public_accounts.push(*account);
            }
        }

        self.rpc_api
            .get_updated_public_accounts(&mismatched_public_accounts)
            .await
            .map_err(ClientError::RpcError)
    }

    /// Applies the changes received from the sync response to the notes and transactions tracked
    /// by the client and updates the `note_updates` accordingly.
    ///
    /// This method uses the callbacks provided to the [`StateSync`] component to check if the
    /// updates received are relevant to the client.
    ///
    /// The note updates might include:
    /// * New notes that we received from the node and might be relevant to the client.
    /// * Tracked expected notes that were committed in the block.
    /// * Tracked notes that were being processed by a transaction that got committed.
    /// * Tracked notes that were nullified by an external transaction.
    async fn note_state_sync(
        &self,
        note_updates: &mut NoteUpdateTracker,
        note_inclusions: Vec<CommittedNote>,
        block_header: &BlockHeader,
    ) -> Result<bool, ClientError> {
        let public_note_ids: Vec<NoteId> = note_inclusions
            .iter()
            .filter_map(|note| (!note.metadata().is_private()).then_some(*note.note_id()))
            .collect();

        let mut found_relevant_note = false;

        // Process note inclusions
        let new_public_notes = self.fetch_public_note_details(&public_note_ids).await?;
        for committed_note in note_inclusions {
            let public_note = new_public_notes.get(committed_note.note_id()).cloned();

            if (self.on_note_received)(committed_note.clone(), public_note.clone()).await? {
                found_relevant_note = true;

                note_updates.apply_committed_note_state_transitions(
                    &committed_note,
                    public_note,
                    block_header,
                )?;
            }
        }

        Ok(found_relevant_note)
    }

    /// Queries the node for all received notes that aren't being locally tracked in the client.
    ///
    /// The client can receive metadata for private notes that it's not tracking. In this case,
    /// notes are ignored for now as they become useless until details are imported.
    async fn fetch_public_note_details(
        &self,
        query_notes: &[NoteId],
    ) -> Result<BTreeMap<NoteId, InputNoteRecord>, ClientError> {
        if query_notes.is_empty() {
            return Ok(BTreeMap::new());
        }
        info!("Getting note details for notes that are not being tracked.");

        let return_notes = self.rpc_api.get_public_note_records(query_notes, None).await?;

        Ok(return_notes.into_iter().map(|note| (note.id(), note)).collect())
    }

    /// Collects the nullifier tags for the notes that were updated in the sync response and uses
    /// the `check_nullifiers_by_prefix` endpoint to check if there are new nullifiers for these
    /// notes. It then processes the nullifiers to apply the state transitions on the note updates.
    ///
    /// The `state_sync_update` field will be updated to track the new discarded transactions.
    async fn sync_nullifiers(
        &self,
        state_sync_update: &mut StateSyncUpdate,
        current_block_num: BlockNumber,
    ) -> Result<(), ClientError> {
        // To receive information about added nullifiers, we reduce them to the higher 16 bits
        // Note that besides filtering by nullifier prefixes, the node also filters by block number
        // (it only returns nullifiers from current_block_num until
        // response.block_header.block_num())

        // Check for new nullifiers for input notes that were updated
        let nullifiers_tags: Vec<u16> = state_sync_update
            .note_updates
            .unspent_nullifiers()
            .map(|nullifier| nullifier.prefix())
            .collect();

        let mut new_nullifiers = self
            .rpc_api
            .check_nullifiers_by_prefix(&nullifiers_tags, current_block_num)
            .await?;

        // Discard nullifiers that are newer than the current block (this might happen if the block
        // changes between the sync_state and the check_nullifier calls)
        new_nullifiers.retain(|update| update.block_num <= state_sync_update.block_num.as_u32());

        // Process nullifiers and track the updates of local tracked transactions that were
        // discarded because the notes that they were processing were nullified by an
        // another transaction.
        let mut discarded_transactions = vec![];

        for nullifier_update in new_nullifiers {
            let discarded_transaction =
                state_sync_update.note_updates.apply_nullifiers_state_transitions(
                    &nullifier_update,
                    state_sync_update.transaction_updates.committed_transactions(),
                )?;

            if let Some(transaction_id) = discarded_transaction {
                discarded_transactions.push(transaction_id);
            }
        }

        let transaction_updates = TransactionUpdates::new(vec![], discarded_transactions, vec![]);
        state_sync_update.transaction_updates.extend(transaction_updates);

        Ok(())
    }

    /// Updates committed unverified notes. These could be notes that were
    /// imported with an inclusion proof, but its block header isn't tracked.
    ///
    /// The method will request the block header and also update the chain MMR
    /// with the new peaks and authentication nodes.
    async fn update_unverified_notes(
        &self,
        state_sync_update: &mut StateSyncUpdate,
        current_chain_mmr: &mut ChainMmr,
    ) -> Result<(), ClientError> {
        let missing_block_nums = state_sync_update
            .note_updates
            .updated_input_notes()
            .filter_map(|note| {
                if let InputNoteState::Unverified(state) = note.inner().state() {
                    Some(state.inclusion_proof.location().block_num())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for block_num in missing_block_nums {
            let block_header = if let Some(block) = current_chain_mmr.get_block(block_num) {
                block.clone()
            } else {
                let current_partial_mmr = current_chain_mmr.partial_mmr_mut();

                let (block_header, path_nodes) =
                    fetch_block_header(self.rpc_api.clone(), block_num, current_partial_mmr)
                        .await?;

                state_sync_update.block_updates.extend(BlockUpdates::new(
                    vec![(block_header.clone(), true, current_partial_mmr.peaks())],
                    path_nodes,
                ));

                block_header
            };

            state_sync_update
                .note_updates
                .apply_block_header_state_transitions(&block_header)?;
        }

        Ok(())
    }
}

// HELPERS
// ================================================================================================

/// Applies changes to the current MMR structure, returns the updated [`MmrPeaks`] and the
/// authentication nodes for leaves we track.
fn apply_mmr_changes(
    new_block: &BlockHeader,
    new_block_has_relevant_notes: bool,
    current_partial_mmr: &mut PartialMmr,
    mmr_delta: MmrDelta,
) -> Result<(MmrPeaks, Vec<(InOrderIndex, Digest)>), ClientError> {
    // Apply the MMR delta to bring MMR to forest equal to chain tip
    let mut new_authentication_nodes: Vec<(InOrderIndex, Digest)> =
        current_partial_mmr.apply(mmr_delta).map_err(StoreError::MmrError)?;

    let new_peaks = current_partial_mmr.peaks();

    new_authentication_nodes
        .append(&mut current_partial_mmr.add(new_block.commitment(), new_block_has_relevant_notes));

    Ok((new_peaks, new_authentication_nodes))
}

// DEFAULT CALLBACK IMPLEMENTATIONS
// ================================================================================================

/// Default implementation of the [`OnNoteReceived`] callback. It queries the store for the
/// committed note to check if it's relevant. If the note wasn't being tracked but it came in the
/// sync response it may be a new public note, in that case we use the [`NoteScreener`] to check its
/// relevance.
pub async fn on_note_received(
    store: Arc<dyn Store>,
    committed_note: CommittedNote,
    public_note: Option<InputNoteRecord>,
) -> Result<bool, ClientError> {
    let note_id = *committed_note.note_id();
    let note_screener = NoteScreener::new(store.clone());

    if !store.get_input_notes(NoteFilter::Unique(note_id)).await?.is_empty()
        || !store.get_output_notes(NoteFilter::Unique(note_id)).await?.is_empty()
    {
        // The note is being tracked by the client so it is relevant
        Ok(true)
    } else if let Some(public_note) = public_note {
        // The note is not being tracked by the client and is public so we can screen it
        let new_note_relevance = note_screener
            .check_relevance(&public_note.try_into().expect("Public notes should contain metadata"))
            .await?;

        Ok(!new_note_relevance.is_empty())
    } else {
        // The note is not being tracked by the client and is private so we can't determine if it
        // is relevant
        Ok(false)
    }
}

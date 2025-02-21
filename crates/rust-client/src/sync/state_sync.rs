use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{future::Future, pin::Pin};

use miden_objects::{
    account::{Account, AccountHeader, AccountId},
    block::BlockHeader,
    crypto::merkle::{InOrderIndex, MmrDelta, MmrPeaks, PartialMmr},
    note::{NoteId, NoteTag},
    Digest,
};
use tracing::info;

use super::{block_header::BlockUpdates, get_nullifier_prefix, StateSyncUpdate};
use crate::{
    account::AccountUpdates,
    note::{NoteScreener, NoteUpdates},
    rpc::{
        domain::{note::CommittedNote, nullifier::NullifierUpdate, transaction::TransactionUpdate},
        NodeRpcClient,
    },
    store::{InputNoteRecord, NoteFilter, OutputNoteRecord, Store, StoreError},
    transaction::TransactionUpdates,
    ClientError,
};

// SYNC CALLBACKS
// ================================================================================================

/// Callback that gets executed when a new note inclusion is received as part of the sync response.
/// It receives the committed note received from the network and the input note state and an
/// optional note record that corresponds to the state of the note in the network (only if the note
/// is public).
///
/// It returns a boolean indicating if the received note update is relevant.
/// If the return value is `false`, it gets discarded. If it is `true`, the update gets committed to
/// the client's store.
pub type OnNoteReceived = Box<
    dyn Fn(
        CommittedNote,
        Option<InputNoteRecord>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, ClientError>>>>,
>;

/// Callback to be executed when a nullifier is received as part of the the sync response. It
/// receives the nullifier update received from the network.
///
/// It returns a boolean indicating if the received note update is relevant
/// If the return value is `false`, it gets discarded. If it is `true`, the update gets committed to
/// the client's store.
pub type OnNullifierReceived =
    Box<dyn Fn(NullifierUpdate) -> Pin<Box<dyn Future<Output = Result<bool, ClientError>>>>>;

// STATE SYNC
// ================================================================================================

/// The state sync components encompasses the client's sync logic.
///
/// When created it receives callbacks that will be executed when a new note inclusion or a
/// nullifier is received in the sync response.
///
///
///  current state of the client's relevant elements (block, accounts,
/// notes, etc). It is then used to requset updates from the node and apply them to the relevant
/// elements. The updates are then returned and can be applied to the store to persist the changes.
pub struct StateSync {
    /// The RPC client used to communicate with the node.
    rpc_api: Arc<dyn NodeRpcClient + Send>,
    /// Callback to be executed when a new note inclusion is received.
    on_note_received: OnNoteReceived,
    /// Callback to be executed when a nullifier is received.
    on_nullifier_received: OnNullifierReceived,
}

impl StateSync {
    /// Creates a new instance of the state sync component.
    ///
    /// # Arguments
    ///
    /// * `rpc_api` - The RPC client used to communicate with the node.
    /// * `on_note_received` - A callback to be executed when a new note inclusion is received.
    /// * `on_nullifier_received` - A callback to be executed when a nullifier is received.
    pub fn new(
        rpc_api: Arc<dyn NodeRpcClient + Send>,
        on_note_received: OnNoteReceived,
        on_nullifier_received: OnNullifierReceived,
    ) -> Self {
        Self {
            rpc_api,
            on_note_received,
            on_nullifier_received,
        }
    }

    /// Syncs the state of the client with the chain tip of the node, returning the updates that
    /// should be applied to the store.
    ///
    /// During the sync process, the client will go through the following steps:
    /// 1. A request is sent to the node to get the state updates. This request includes tracked
    ///    account IDs and the tags of notes that might have changed or that might be of interest to
    ///    the client.
    /// 2. A response is received with the current state of the network. The response includes
    ///    information about new/committed/consumed notes, updated accounts, and committed
    ///    transactions.
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
    /// * `current_partial_mmr` - The current partial MMR.
    /// * `accounts` - All the headers of tracked accounts.
    /// * `note_tags` - The note tags to be used in the sync state request.
    /// * `unspent_input_notes` - The current state of unspent input notes tracked by the client.
    /// * `unspent_output_notes` - The current state of unspent output notes tracked by the client.
    pub async fn sync_state(
        &self,
        mut current_partial_mmr: PartialMmr,
        accounts: Vec<AccountHeader>,
        note_tags: Vec<NoteTag>,
        unspent_input_notes: Vec<InputNoteRecord>,
        unspent_output_notes: Vec<OutputNoteRecord>,
    ) -> Result<StateSyncUpdate, ClientError> {
        let unspent_nullifiers = unspent_input_notes
            .iter()
            .map(InputNoteRecord::nullifier)
            .chain(unspent_output_notes.iter().filter_map(OutputNoteRecord::nullifier));

        // To receive information about added nullifiers, we reduce them to the higher 16 bits
        // Note that besides filtering by nullifier prefixes, the node also filters by block number
        // (it only returns nullifiers from current_block_num until
        // response.block_header.block_num())
        let mut nullifiers_tags: Vec<u16> =
            unspent_nullifiers.map(|nullifier| get_nullifier_prefix(&nullifier)).collect();

        let mut state_sync_update = StateSyncUpdate {
            note_updates: NoteUpdates::new(unspent_input_notes, unspent_output_notes),
            ..Default::default()
        };

        while self
            .sync_state_step(
                &mut state_sync_update,
                &mut current_partial_mmr,
                &accounts,
                &note_tags,
                &nullifiers_tags,
            )
            .await?
        {
            // New nullfiers should be added for new untracked notes that were added in previous
            // steps
            nullifiers_tags.append(
                &mut state_sync_update
                    .note_updates
                    .updated_input_notes()
                    .filter(|note| {
                        note.is_committed()
                            && !nullifiers_tags.contains(&get_nullifier_prefix(&note.nullifier()))
                    })
                    .map(|note| get_nullifier_prefix(&note.nullifier()))
                    .collect::<Vec<_>>(),
            );
        }

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
        current_partial_mmr: &mut PartialMmr,
        accounts: &[AccountHeader],
        note_tags: &[NoteTag],
        nullifiers_tags: &[u16],
    ) -> Result<bool, ClientError> {
        let current_block_num = (u32::try_from(current_partial_mmr.num_leaves() - 1)
            .expect("The number of leaves in the MMR should be greater than 0 and less than 2^32"))
        .into();
        let account_ids: Vec<AccountId> = accounts.iter().map(AccountHeader::id).collect();

        let response = self
            .rpc_api
            .sync_state(current_block_num, &account_ids, note_tags, nullifiers_tags)
            .await?;

        state_sync_update.block_num = response.block_header.block_num();

        // We don't need to continue if the chain has not advanced, there are no new changes
        if response.block_header.block_num() == current_block_num {
            return Ok(false);
        }

        let account_updates =
            self.account_state_sync(accounts, &response.account_hash_updates).await?;

        state_sync_update.account_updates = account_updates;

        let (found_relevant_note, transaction_updates) = self
            .note_state_sync(
                &mut state_sync_update.note_updates,
                response.note_inclusions,
                response.transactions,
                response.nullifiers,
                &response.block_header,
            )
            .await?;

        state_sync_update.transaction_updates.extend(transaction_updates);

        let (new_mmr_peaks, new_authentication_nodes) = apply_mmr_changes(
            &response.block_header,
            found_relevant_note,
            current_partial_mmr,
            response.mmr_delta,
        )?;

        state_sync_update.block_updates.extend(BlockUpdates::new(
            vec![(response.block_header, found_relevant_note, new_mmr_peaks)],
            new_authentication_nodes,
        ));

        if response.chain_tip == response.block_header.block_num() {
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
    /// * Private accounts that have been marked as mismatched because the current hash doesn't
    ///   match the one received from the node. The client will need to handle these cases as they
    ///   could be a stale account state or a reason to lock the account.
    async fn account_state_sync(
        &self,
        accounts: &[AccountHeader],
        account_hash_updates: &[(AccountId, Digest)],
    ) -> Result<AccountUpdates, ClientError> {
        let (public_accounts, private_accounts): (Vec<_>, Vec<_>) =
            accounts.iter().partition(|account_header| account_header.id().is_public());

        let updated_public_accounts =
            self.get_updated_public_accounts(account_hash_updates, &public_accounts).await?;

        let mismatched_private_accounts = account_hash_updates
            .iter()
            .filter(|(account_id, digest)| {
                private_accounts
                    .iter()
                    .any(|account| account.id() == *account_id && &account.hash() != digest)
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

        for (id, hash) in account_updates {
            // check if this updated account state is tracked by the client
            if let Some(account) = current_public_accounts
                .iter()
                .find(|acc| *id == acc.id() && *hash != acc.hash())
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
    /// by the client and updates the `state_sync_update` accordingly.
    ///
    /// This method uses the callbacks provided to the [`StateSync`] component to check if the
    /// updates received are relevant to the client.
    ///
    /// The note updates might include:
    /// * New notes that we received from the node and might be relevant to the client.
    /// * Tracked expected notes that were committed in the block.
    /// * Tracked notes that were being processed by a transaction that got committed.
    /// * Tracked notes that were nullified by an external transaction.
    ///
    /// The transaction updates might include:
    /// * Transactions that were committed in the block. Some of these might me tracked by the
    ///   client and need to be marked as committed.
    /// * Local tracked transactions that were discarded because the notes that they were processing
    ///   were nullified by an another transaction.
    async fn note_state_sync(
        &self,
        note_updates: &mut NoteUpdates,
        note_inclusions: Vec<CommittedNote>,
        transactions: Vec<TransactionUpdate>,
        nullifiers: Vec<NullifierUpdate>,
        block_header: &BlockHeader,
    ) -> Result<(bool, TransactionUpdates), ClientError> {
        let public_note_ids: Vec<NoteId> = note_inclusions
            .iter()
            .filter_map(|note| (!note.metadata().is_private()).then_some(*note.note_id()))
            .collect();

        let mut found_relevant_note = false;
        let mut discarded_transactions = vec![];

        // Process note inclusions
        let new_public_notes =
            Arc::new(self.fetch_public_note_details(&public_note_ids, block_header).await?);
        for committed_note in note_inclusions {
            let public_note = new_public_notes
                .iter()
                .find(|note| &note.id() == committed_note.note_id())
                .cloned();
            if (self.on_note_received)(committed_note.clone(), public_note.clone()).await? {
                found_relevant_note = true;

                if let Some(public_note) = public_note {
                    note_updates.insert_updates(Some(public_note), None);
                }

                note_updates
                    .apply_committed_note_state_transitions(&committed_note, block_header)?;
            }
        }

        // Process nullifiers
        for nullifier_update in nullifiers {
            if (self.on_nullifier_received)(nullifier_update.clone()).await? {
                let discarded_transaction = note_updates
                    .apply_nullifiers_state_transitions(&nullifier_update, &transactions)?;

                if let Some(transaction_id) = discarded_transaction {
                    discarded_transactions.push(transaction_id);
                }
            }
        }

        Ok((
            found_relevant_note,
            TransactionUpdates::new(transactions, discarded_transactions),
        ))
    }

    /// Queries the node for all received notes that aren't being locally tracked in the client.
    ///
    /// The client can receive metadata for private notes that it's not tracking. In this case,
    /// notes are ignored for now as they become useless until details are imported.
    async fn fetch_public_note_details(
        &self,
        query_notes: &[NoteId],
        block_header: &BlockHeader,
    ) -> Result<Vec<InputNoteRecord>, ClientError> {
        if query_notes.is_empty() {
            return Ok(vec![]);
        }
        info!("Getting note details for notes that are not being tracked.");

        let mut return_notes = self.rpc_api.get_public_note_records(query_notes, None).await?;

        for note in &mut return_notes {
            note.block_header_received(block_header)?;
        }

        Ok(return_notes)
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
        .append(&mut current_partial_mmr.add(new_block.hash(), new_block_has_relevant_notes));

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

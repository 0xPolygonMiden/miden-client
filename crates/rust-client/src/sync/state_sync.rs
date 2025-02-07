use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{future::Future, pin::Pin};

use miden_objects::{
    account::{Account, AccountHeader, AccountId},
    block::{BlockHeader, BlockNumber},
    crypto::merkle::{InOrderIndex, MmrDelta, MmrPeaks, PartialMmr},
    note::{NoteId, NoteInclusionProof, NoteTag, Nullifier},
    transaction::TransactionId,
    Digest,
};
use tracing::info;

use super::{block_header::BlockUpdates, get_nullifier_prefix, SyncSummary};
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

// STATE SYNC UPDATE
// ================================================================================================

#[derive(Default)]
/// Contains all information needed to apply the update in the store after syncing with the node.
pub struct StateSyncUpdate {
    /// The block number of the last block that was synced.
    pub block_num: BlockNumber,
    /// New blocks and authentication nodes.
    pub block_updates: BlockUpdates,
    /// New and updated notes to be upserted in the store.
    pub note_updates: NoteUpdates,
    /// Committed and discarded transactions after the sync.
    pub transaction_updates: TransactionUpdates,
    /// Public account updates and mismatched private accounts after the sync.
    pub account_updates: AccountUpdates,
}

impl From<&StateSyncUpdate> for SyncSummary {
    fn from(value: &StateSyncUpdate) -> Self {
        SyncSummary::new(
            value.block_num,
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

// SYNC CALLBACKS
// ================================================================================================

/// Callback to be executed when a new note inclusion is received in the sync response. It receives
/// the committed note received from the node, the block header in which the note was included and
/// the list of public notes that were included in the block.
///
/// It returns two optional notes (one input and one output) that should be updated in the store and
/// a flag indicating if the block is relevant to the client.
pub type OnNoteReceived = Box<
    dyn Fn(
        CommittedNote,
        Option<InputNoteRecord>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, ClientError>>>>,
>;

/// Callback to be executed when a nullifier is received in the sync response. It receives the
/// nullifier update received from the node and the list of transaction updates that were committed
/// in the block.
///
/// It returns two optional notes (one input and one output) that should be updated in the store and
/// an optional transaction ID if a transaction should be discarded.
pub type OnNullifierReceived =
    Box<dyn Fn(NullifierUpdate) -> Pin<Box<dyn Future<Output = Result<bool, ClientError>>>>>;

// STATE SYNC
// ================================================================================================

/// The state sync components encompasses the client's sync logic.
///
/// When created it receives the current state of the client's relevant elements (block, accounts,
/// notes, etc). It is then used to requset updates from the node and apply them to the relevant
/// elements. The updates are then returned and can be applied to the store to persist the changes.
pub struct StateSync {
    /// The RPC client used to communicate with the node.
    rpc_api: Arc<dyn NodeRpcClient + Send>,
    /// Callback to be executed when a new note inclusion is received.
    on_note_received: OnNoteReceived,
    /// Callback to be executed when a nullifier is received.
    on_nullifier_received: OnNullifierReceived,
    /// The state sync update that will be returned after the sync process is completed. It
    /// agregates all the updates that come from each sync step.
    state_sync_update: StateSyncUpdate,
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
            state_sync_update: StateSyncUpdate::default(),
        }
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
        &mut self,
        current_partial_mmr: &mut PartialMmr,
        accounts: &[AccountHeader],
        note_tags: &[NoteTag],
        unspent_nullifiers: &[Nullifier],
    ) -> Result<bool, ClientError> {
        let current_block_num = (current_partial_mmr.num_leaves() as u32 - 1).into();
        let account_ids: Vec<AccountId> = accounts.iter().map(|acc| acc.id()).collect();

        // To receive information about added nullifiers, we reduce them to the higher 16 bits
        // Note that besides filtering by nullifier prefixes, the node also filters by block number
        // (it only returns nullifiers from current_block_num until
        // response.block_header.block_num())
        let nullifiers_tags: Vec<u16> =
            unspent_nullifiers.iter().map(get_nullifier_prefix).collect();

        let response = self
            .rpc_api
            .sync_state(current_block_num, &account_ids, note_tags, &nullifiers_tags)
            .await?;

        self.state_sync_update.block_num = response.block_header.block_num();

        // We don't need to continue if the chain has not advanced, there are no new changes
        if response.block_header.block_num() == current_block_num {
            return Ok(false);
        }

        self.account_state_sync(accounts, &response.account_hash_updates).await?;

        let found_relevant_note = self
            .note_state_sync(
                response.note_inclusions,
                response.transactions,
                response.nullifiers,
                response.block_header,
            )
            .await?;

        let (new_mmr_peaks, new_authentication_nodes) = apply_mmr_changes(
            response.block_header,
            found_relevant_note,
            current_partial_mmr,
            response.mmr_delta,
        )
        .await?;

        self.state_sync_update.block_updates.extend(BlockUpdates {
            block_headers: vec![(response.block_header, found_relevant_note, new_mmr_peaks)],
            new_authentication_nodes,
        });

        if response.chain_tip == response.block_header.block_num() {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    /// Syncs the state of the client with the chain tip of the node, returning the updates that
    /// should be applied to the store.
    ///
    /// # Arguments
    /// * `current_block` - The latest tracked block header.
    /// * `current_block_has_relevant_notes` - A flag indicating if the current block has notes that
    ///   are relevant to the client. This is used to determine whether new MMR authentication nodes
    ///   are stored for this block.
    /// * `current_partial_mmr` - The current partial MMR.
    /// * `accounts` - The headers of tracked accounts.
    /// * `note_tags` - The note tags to be used in the sync state request.
    /// * `unspent_nullifiers` - The nullifiers of tracked notes that haven't been consumed.
    pub async fn sync_state(
        mut self,
        mut current_partial_mmr: PartialMmr,
        accounts: Vec<AccountHeader>,
        note_tags: Vec<NoteTag>,
        unspent_input_notes: Vec<InputNoteRecord>,
        unspent_output_notes: Vec<OutputNoteRecord>,
    ) -> Result<StateSyncUpdate, ClientError> {
        let mut unspent_nullifiers: Vec<Nullifier> = unspent_input_notes
            .iter()
            .map(|note| note.nullifier())
            .chain(unspent_output_notes.iter().filter_map(|note| note.nullifier()))
            .collect();

        self.state_sync_update.note_updates =
            NoteUpdates::new(unspent_input_notes, unspent_output_notes);

        while self
            .sync_state_step(&mut current_partial_mmr, &accounts, &note_tags, &unspent_nullifiers)
            .await?
        {
            // New nullfiers should be added for new untracked notes that were added in previous
            // steps
            unspent_nullifiers.append(
                &mut self
                    .state_sync_update
                    .note_updates
                    .updated_input_notes()
                    .filter(|note| {
                        note.is_committed() && !unspent_nullifiers.contains(&note.nullifier())
                    })
                    .map(|note| note.nullifier())
                    .collect::<Vec<_>>(),
            );
        }

        Ok(self.state_sync_update)
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Compares the state of tracked accounts with the updates received from the node and updates
    /// the `state_sync_update` with the details of
    /// the accounts that need to be updated.
    ///
    /// When a mismatch is detected, two scenarios are possible:
    /// * If the account is public, the component will request the node for the updated account
    ///   details.
    /// * If the account is private it will be marked as mismatched and the client will need to
    ///   handle it (it could be a stale account state or a reason to lock the account).
    async fn account_state_sync(
        &mut self,
        accounts: &[AccountHeader],
        account_hash_updates: &[(AccountId, Digest)],
    ) -> Result<(), ClientError> {
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
            .cloned()
            .collect::<Vec<_>>();

        self.state_sync_update
            .account_updates
            .extend(AccountUpdates::new(updated_public_accounts, mismatched_private_accounts));

        Ok(())
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
            // check if this updated account is tracked by the client
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
    /// by the client and updates the
    /// `state_sync_update` accordingly.
    ///
    /// This method uses the callbacks provided to the [StateSync] component to apply the changes.
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
    async fn note_state_sync(
        &mut self,
        note_inclusions: Vec<CommittedNote>,
        transactions: Vec<TransactionUpdate>,
        nullifiers: Vec<NullifierUpdate>,
        block_header: BlockHeader,
    ) -> Result<bool, ClientError> {
        let public_note_ids: Vec<NoteId> = note_inclusions
            .iter()
            .filter_map(|note| (!note.metadata().is_private()).then_some(*note.note_id()))
            .collect();

        let mut found_relevant_note = false;

        // Process note inclusions
        let new_public_notes =
            Arc::new(self.fetch_public_note_details(&public_note_ids, &block_header).await?);
        for committed_note in note_inclusions {
            let public_note = new_public_notes
                .iter()
                .find(|note| &note.id() == committed_note.note_id())
                .cloned();
            if (self.on_note_received)(committed_note.clone(), public_note.clone()).await? {
                found_relevant_note = true;

                if let Some(public_note) = public_note {
                    self.state_sync_update.note_updates.insert_updates(Some(public_note), None);
                }

                committed_state_transions(
                    &mut self.state_sync_update.note_updates,
                    committed_note,
                    block_header,
                )
                .await?;
            }
        }

        // Process nullifiers
        for nullifier_update in nullifiers {
            if (self.on_nullifier_received)(nullifier_update.clone()).await? {
                let discarded_transaction = nullfier_state_transitions(
                    &mut self.state_sync_update.note_updates,
                    nullifier_update,
                    &transactions,
                )
                .await?;

                if let Some(transaction_id) = discarded_transaction {
                    self.state_sync_update
                        .transaction_updates
                        .discarded_transaction(transaction_id);
                }
            }
        }

        self.state_sync_update
            .transaction_updates
            .extend(TransactionUpdates::new(transactions, vec![]));

        Ok(found_relevant_note)
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

        for note in return_notes.iter_mut() {
            note.block_header_received(*block_header)?;
        }

        Ok(return_notes)
    }
}

// HELPERS
// ================================================================================================

/// Applies changes to the current MMR structure, returns the updated [MmrPeaks] and the
/// authentication nodes for leaves we track.
async fn apply_mmr_changes(
    new_block: BlockHeader,
    new_block_has_relevant_notes: bool,
    current_partial_mmr: &mut PartialMmr,
    mmr_delta: MmrDelta,
) -> Result<(MmrPeaks, Vec<(InOrderIndex, Digest)>), ClientError> {
    // First, apply curent_block to the MMR. This is needed as the MMR delta received from the
    // node doesn't contain the request block itself.
    // let new_authentication_nodes = current_partial_mmr
    //     .add(current_block.hash(), current_block_has_relevant_notes)
    //     .into_iter();

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

/// Default implementation of the [OnNoteReceived] callback. It queries the store for the committed
/// note and updates it accordingly. If the note wasn't being tracked but it came in the sync
/// response, it is also returned so it can be inserted in the store. The method also returns a
/// flag indicating if the block is relevant to the client.
async fn committed_state_transions(
    note_updates: &mut NoteUpdates,
    committed_note: CommittedNote,
    block_header: BlockHeader,
) -> Result<(), ClientError> {
    let inclusion_proof = NoteInclusionProof::new(
        block_header.block_num(),
        committed_note.note_index(),
        committed_note.merkle_path().clone(),
    )?;

    if let Some(input_note_record) = note_updates.get_input_note_by_id(*committed_note.note_id()) {
        // The note belongs to our locally tracked set of input notes
        input_note_record
            .inclusion_proof_received(inclusion_proof.clone(), committed_note.metadata())?;
        input_note_record.block_header_received(block_header)?;
    }

    if let Some(output_note_record) = note_updates.get_output_note_by_id(*committed_note.note_id())
    {
        // The note belongs to our locally tracked set of output notes
        output_note_record.inclusion_proof_received(inclusion_proof.clone())?;
    }

    Ok(())
}

/// Default implementation of the [OnNullifierReceived] callback. It queries the store for the notes
/// that match the nullifier and updates the note records accordingly. It also returns an optional
/// transaction ID that should be discarded.
async fn nullfier_state_transitions(
    note_updates: &mut NoteUpdates,
    nullifier_update: NullifierUpdate,
    transaction_updates: &[TransactionUpdate],
) -> Result<Option<TransactionId>, ClientError> {
    let mut discarded_transaction = None;

    if let Some(input_note_record) =
        note_updates.get_input_note_by_nullifier(nullifier_update.nullifier)
    {
        if let Some(consumer_transaction) = transaction_updates.iter().find(|t| {
            input_note_record
                .consumer_transaction_id()
                .map_or(false, |id| id == &t.transaction_id)
        }) {
            // The note was being processed by a local transaction that just got committed
            input_note_record.transaction_committed(
                consumer_transaction.transaction_id,
                consumer_transaction.block_num,
            )?;
        } else {
            // The note was consumed by an external transaction
            if let Some(id) = input_note_record.consumer_transaction_id() {
                // The note was being processed by a local transaction that didn't end up being
                // committed so it should be discarded
                discarded_transaction.replace(*id);
            }
            input_note_record
                .consumed_externally(nullifier_update.nullifier, nullifier_update.block_num)?;
        }
    }

    if let Some(output_note_record) =
        note_updates.get_output_note_by_nullifier(nullifier_update.nullifier)
    {
        output_note_record
            .nullifier_received(nullifier_update.nullifier, nullifier_update.block_num)?;
    }

    Ok(discarded_transaction)
}

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

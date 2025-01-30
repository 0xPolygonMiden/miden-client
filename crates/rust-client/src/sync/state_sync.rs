use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{future::Future, pin::Pin};

use miden_objects::{
    account::{Account, AccountHeader, AccountId},
    block::{BlockHeader, BlockNumber},
    crypto::merkle::{InOrderIndex, MmrDelta, MmrPeaks, PartialMmr},
    note::{NoteId, NoteInclusionProof, NoteTag, Nullifier},
    Digest,
};
use tracing::info;

use super::{block_header::BlockUpdates, get_nullifier_prefix, NoteTagRecord, SyncSummary};
use crate::{
    account::AccountUpdates,
    note::{NoteScreener, NoteUpdates},
    rpc::{
        domain::{note::CommittedNote, nullifier::NullifierUpdate, transaction::TransactionUpdate},
        NodeRpcClient,
    },
    store::{InputNoteRecord, NoteFilter, Store, StoreError},
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
    /// Tag records that are no longer relevant.
    pub tags_to_remove: Vec<NoteTagRecord>,
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
/// the committed note received from the node and the block header in which the note was included.
/// It returns the note updates that should be applied to the store and a list of public note IDs
/// that should be queried from the node and start being tracked.
pub type OnNoteReceived = Box<
    dyn Fn(
        NoteUpdates,
        CommittedNote,
        BlockHeader,
    ) -> Pin<Box<dyn Future<Output = Result<(NoteUpdates, Vec<NoteId>), ClientError>>>>,
>;

/// Callback to be executed when a transaction is marked committed in the sync response. It receives
/// the transaction update received from the node. It returns the note updates and transaction
/// updates that should be applied to the store as a result of the transaction being committed.
pub type OnTransactionCommitted = Box<
    dyn Fn(
        NoteUpdates,
        TransactionUpdate,
    )
        -> Pin<Box<dyn Future<Output = Result<(NoteUpdates, TransactionUpdates), ClientError>>>>,
>;

/// Callback to be executed when a nullifier is received in the sync response. If a note was
/// consumed by a committed transaction provided in the [OnTransactionCommitted] callback, its
/// nullifier will not be passed to this callback. It receives the nullifier update received from
/// the node. It returns the note updates and transaction updates that should be applied to the
/// store as a result of the nullifier being received.
pub type OnNullifierReceived = Box<
    dyn Fn(
        NoteUpdates,
        NullifierUpdate,
    )
        -> Pin<Box<dyn Future<Output = Result<(NoteUpdates, TransactionUpdates), ClientError>>>>,
>;

pub type OnBlockHeaderReceived = Box<
    dyn Fn(
        BlockHeader,
        NoteUpdates,
        BlockHeader,
        bool,
        PartialMmr,
        MmrDelta,
    ) -> Pin<Box<dyn Future<Output = Result<(BlockUpdates, PartialMmr), ClientError>>>>,
>;

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
    /// Callback to be executed when a transaction is committed.
    on_transaction_committed: OnTransactionCommitted,
    /// Callback to be executed when a nullifier is received.
    on_nullifier_received: OnNullifierReceived,
    /// Callback to be executed when a block header is received.
    on_block_received: OnBlockHeaderReceived,
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
    /// * `on_committed_transaction` - A callback to be executed when a transaction is committed.
    /// * `on_nullifier_received` - A callback to be executed when a nullifier is received.
    /// * `on_block_received` - A callback to be executed when a block header is received.
    pub fn new(
        rpc_api: Arc<dyn NodeRpcClient + Send>,
        on_note_received: OnNoteReceived,
        on_transaction_committed: OnTransactionCommitted,
        on_nullifier_received: OnNullifierReceived,
        on_block_received: OnBlockHeaderReceived,
    ) -> Self {
        Self {
            rpc_api,
            on_note_received,
            on_transaction_committed,
            on_nullifier_received,
            on_block_received,
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
        current_block: BlockHeader,
        current_block_has_relevant_notes: bool,
        current_partial_mmr: &mut PartialMmr,
        accounts: &[AccountHeader],
        note_tags: &[NoteTag],
        unspent_nullifiers: &[Nullifier],
    ) -> Result<bool, ClientError> {
        let current_block_num = current_block.block_num();
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

        self.note_state_sync(
            response.note_inclusions,
            response.transactions,
            response.nullifiers,
            response.block_header,
        )
        .await?;

        let (new_block_updates, new_partial_mmr) = (self.on_block_received)(
            response.block_header,
            self.state_sync_update.note_updates.clone(),
            current_block,
            current_block_has_relevant_notes,
            current_partial_mmr.clone(),
            response.mmr_delta,
        )
        .await?;

        self.state_sync_update.block_updates.extend(new_block_updates);
        *current_partial_mmr = new_partial_mmr;

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
        mut current_block: BlockHeader,
        mut current_block_has_relevant_notes: bool,
        mut current_partial_mmr: PartialMmr,
        accounts: Vec<AccountHeader>,
        note_tags: Vec<NoteTag>,
        unspent_nullifiers: Vec<Nullifier>,
    ) -> Result<StateSyncUpdate, ClientError> {
        loop {
            if !self
                .sync_state_step(
                    current_block,
                    current_block_has_relevant_notes,
                    &mut current_partial_mmr,
                    &accounts,
                    &note_tags,          //TODO: get note tags from notes in the updates
                    &unspent_nullifiers, //TODO: get nullifiers from notes in the updates
                )
                .await?
            {
                return Ok(self.state_sync_update);
            }

            (current_block, current_block_has_relevant_notes, ..) = self
                .state_sync_update
                .block_updates
                .block_headers
                .last()
                .cloned()
                .expect("At least one block header should be present");
        }
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Compares the state of tracked accounts with the updates received from the node and updates the
    /// `state_sync_update` with the details of
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
        mut nullifiers: Vec<NullifierUpdate>,
        block_header: BlockHeader,
    ) -> Result<(), ClientError> {
        let mut public_note_ids = vec![];
        let mut note_updates = self.state_sync_update.note_updates.clone();

        for committed_note in note_inclusions {
            let (new_note_updates, new_note_ids) =
                (self.on_note_received)(note_updates, committed_note, block_header).await?;
            note_updates = new_note_updates;
            public_note_ids.extend(new_note_ids);
        }

        let new_public_notes =
            self.fetch_public_note_details(&public_note_ids, &block_header).await?;

        note_updates.insert_or_ignore_notes(&new_public_notes, &vec![]);

        // We can remove tags from notes that got committed
        let tags_to_remove: Vec<NoteTagRecord> = note_updates
            .updated_input_notes()
            .filter(|note| note.is_committed())
            .map(|note| {
                NoteTagRecord::with_note_source(
                    note.metadata().expect("Committed note should have metadata").tag(),
                    note.id(),
                )
            })
            .collect();

        self.state_sync_update.tags_to_remove.extend(tags_to_remove);

        for transaction_update in transactions {
            let (new_note_updates, new_transaction_update) =
                (self.on_transaction_committed)(note_updates, transaction_update).await?;

            // Remove nullifiers if they were consumed by the transaction
            nullifiers.retain(|nullifier| {
                !new_note_updates
                    .updated_input_notes()
                    .any(|note| note.nullifier() == nullifier.nullifier)
            });

            note_updates = new_note_updates;
            self.state_sync_update.transaction_updates.extend(new_transaction_update);
        }

        for nullifier_update in nullifiers {
            let (new_note_updates, new_transaction_update) =
                (self.on_nullifier_received)(note_updates, nullifier_update).await?;

            note_updates = new_note_updates;
            self.state_sync_update.transaction_updates.extend(new_transaction_update);
        }

        self.state_sync_update.note_updates = note_updates;

        Ok(())
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
    current_block: BlockHeader,
    current_block_has_relevant_notes: bool,
    current_partial_mmr: &mut PartialMmr,
    mmr_delta: MmrDelta,
) -> Result<(MmrPeaks, Vec<(InOrderIndex, Digest)>), ClientError> {
    // First, apply curent_block to the MMR. This is needed as the MMR delta received from the
    // node doesn't contain the request block itself.
    let new_authentication_nodes = current_partial_mmr
        .add(current_block.hash(), current_block_has_relevant_notes)
        .into_iter();

    // Apply the MMR delta to bring MMR to forest equal to chain tip
    let new_authentication_nodes: Vec<(InOrderIndex, Digest)> = current_partial_mmr
        .apply(mmr_delta)
        .map_err(StoreError::MmrError)?
        .into_iter()
        .chain(new_authentication_nodes)
        .collect();

    Ok((current_partial_mmr.peaks(), new_authentication_nodes))
}

// DEFAULT CALLBACK IMPLEMENTATIONS
// ================================================================================================

/// Default implementation of the [OnNoteReceived] callback. It queries the store for the committed
/// note and updates the note records accordingly. If the note is not being tracked, it returns the
/// note ID to be queried from the node so it can be queried from the node and tracked.
pub async fn on_note_received(
    store: Arc<dyn Store>,
    mut note_updates: NoteUpdates,
    committed_note: CommittedNote,
    block_header: BlockHeader,
) -> Result<(NoteUpdates, Vec<NoteId>), ClientError> {
    let inclusion_proof = NoteInclusionProof::new(
        block_header.block_num(),
        committed_note.note_index(),
        committed_note.merkle_path().clone(),
    )?;

    let mut is_tracked_note = false;
    let mut new_note_ids = vec![];

    note_updates.insert_or_ignore_notes(
        &store.get_input_notes(NoteFilter::List(vec![*committed_note.note_id()])).await?,
        &store
            .get_output_notes(NoteFilter::List(vec![*committed_note.note_id()]))
            .await?,
    );

    if let Some(input_note_record) = note_updates.get_input_note_by_id(committed_note.note_id()) {
        // The note belongs to our locally tracked set of input notes
        is_tracked_note = true;
        input_note_record
            .inclusion_proof_received(inclusion_proof.clone(), committed_note.metadata())?;
        input_note_record.block_header_received(block_header)?;
    }

    if let Some(output_note_record) = note_updates.get_output_note_by_id(committed_note.note_id()) {
        // The note belongs to our locally tracked set of output notes
        is_tracked_note = true;
        output_note_record.inclusion_proof_received(inclusion_proof.clone())?;
    }

    if !is_tracked_note {
        // The note is public and we are not tracking it, push to the list of IDs to query
        new_note_ids.push(*committed_note.note_id());
    }

    Ok((note_updates, new_note_ids))
}

/// Default implementation of the [OnTransactionCommitted] callback. It queries the store for the
/// input notes that were consumed by the transaction and updates the note records accordingly. It
/// also returns the committed transaction update to be applied to the store.
pub async fn on_transaction_committed(
    store: Arc<dyn Store>,
    mut note_updates: NoteUpdates,
    transaction_update: TransactionUpdate,
) -> Result<(NoteUpdates, TransactionUpdates), ClientError> {
    let processing_notes = store.get_input_notes(NoteFilter::Processing).await?;
    let consumed_input_notes: Vec<InputNoteRecord> = processing_notes
        .into_iter()
        .filter(|note_record| {
            note_record.consumer_transaction_id() == Some(&transaction_update.transaction_id)
        })
        .collect();

    let consumed_output_notes = store
        .get_output_notes(NoteFilter::Nullifiers(
            consumed_input_notes.iter().map(|n| n.nullifier()).collect(),
        ))
        .await?;

    note_updates.insert_or_ignore_notes(&consumed_input_notes, &consumed_output_notes);

    for store_note in consumed_input_notes {
        let input_note_record = note_updates
            .get_input_note_by_id(&store_note.id())
            .expect("Input note should be present in the note updates after being inserted");

        input_note_record.transaction_committed(
            transaction_update.transaction_id,
            transaction_update.block_num,
        )?;
    }

    for store_note in consumed_output_notes {
        // SAFETY: Output notes were queried from a nullifier list and should have a nullifier
        let nullifier = store_note.nullifier().unwrap();
        let output_note_record = note_updates
            .get_output_note_by_id(&store_note.id())
            .expect("Output note should be present in the note updates after being inserted");
        output_note_record.nullifier_received(nullifier, transaction_update.block_num)?;
    }

    Ok((note_updates, TransactionUpdates::new(vec![transaction_update], vec![])))
}

/// Default implementation of the [OnNullifierReceived] callback. It queries the store for the notes
/// that match the nullifier and updates the note records accordingly. It also returns the
/// transactions that should be discarded as they weren't committed when the nullifier was received.
pub async fn on_nullifier_received(
    store: Arc<dyn Store>,
    mut note_updates: NoteUpdates,
    nullifier_update: NullifierUpdate,
) -> Result<(NoteUpdates, TransactionUpdates), ClientError> {
    let mut discarded_transactions = vec![];

    note_updates.insert_or_ignore_notes(
        &store
            .get_input_notes(NoteFilter::Nullifiers(vec![nullifier_update.nullifier]))
            .await?,
        &store
            .get_output_notes(NoteFilter::Nullifiers(vec![nullifier_update.nullifier]))
            .await?,
    );

    if let Some(input_note_record) =
        note_updates.get_input_note_by_nullifier(nullifier_update.nullifier)
    {
        if input_note_record.is_processing() {
            discarded_transactions.push(
                *input_note_record
                    .consumer_transaction_id()
                    .expect("Processing note should have consumer transaction id"),
            );
        }

        input_note_record
            .consumed_externally(nullifier_update.nullifier, nullifier_update.block_num)?;
    }

    if let Some(output_note_record) =
        note_updates.get_output_note_by_nullifier(nullifier_update.nullifier)
    {
        output_note_record
            .nullifier_received(nullifier_update.nullifier, nullifier_update.block_num)?;
    }

    Ok((note_updates, TransactionUpdates::new(vec![], discarded_transactions)))
}

pub async fn on_block_received(
    store: Arc<dyn Store>,
    new_block_header: BlockHeader,
    note_updates: NoteUpdates,
    current_block_header: BlockHeader,
    current_block_has_relevant_notes: bool,
    mut current_partial_mmr: PartialMmr,
    mmr_delta: MmrDelta,
) -> Result<(BlockUpdates, PartialMmr), ClientError> {
    let (mmr_peaks, new_authentication_nodes) = apply_mmr_changes(
        current_block_header,
        current_block_has_relevant_notes,
        &mut current_partial_mmr,
        mmr_delta,
    )
    .await?;

    let block_relevance =
        check_block_relevance(store.clone(), new_block_header.block_num(), note_updates).await?;

    Ok((
        BlockUpdates {
            block_headers: vec![(new_block_header, block_relevance, mmr_peaks)],
            new_authentication_nodes,
        },
        current_partial_mmr,
    ))
}

/// Checks the relevance of the block by verifying if any of the input notes in the block are
/// relevant to the client. If any of the notes are relevant, the function returns `true`.
pub(crate) async fn check_block_relevance(
    store: Arc<dyn Store>,
    new_block_number: BlockNumber,
    note_updates: NoteUpdates,
) -> Result<bool, ClientError> {
    // We'll only do the check for either incoming public notes or expected input notes as
    // output notes are not really candidates to be consumed here.

    let note_screener = NoteScreener::new(store);

    // Find all relevant Input Notes using the note checker
    for input_note in note_updates.committed_input_notes() {
        if input_note
            .inclusion_proof()
            .is_some_and(|proof| proof.location().block_num() != new_block_number)
        {
            // This note wasn't received in the current block, so it shouldn't be considered
            continue;
        }

        if !note_screener
            .check_relevance(&input_note.try_into().expect("Committed notes should have metadata"))
            .await?
            .is_empty()
        {
            return Ok(true);
        }
    }

    Ok(false)
}

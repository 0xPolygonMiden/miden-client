use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{future::Future, pin::Pin};

use miden_objects::{
    accounts::{Account, AccountHeader, AccountId},
    crypto::merkle::{InOrderIndex, MmrDelta, MmrPeaks, PartialMmr},
    notes::{NoteId, NoteInclusionProof, NoteTag, Nullifier},
    BlockHeader, Digest,
};
use tracing::info;

use super::{get_nullifier_prefix, NoteTagRecord, SyncSummary};
use crate::{
    accounts::AccountUpdates,
    notes::NoteUpdates,
    rpc::{
        domain::{
            notes::CommittedNote, nullifiers::NullifierUpdate, transactions::TransactionUpdate,
        },
        NodeRpcClient,
    },
    store::{InputNoteRecord, NoteFilter, Store, StoreError},
    transactions::TransactionUpdates,
    ClientError,
};

// STATE SYNC UPDATE
// ================================================================================================

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

/// Gives information about the status of the sync process after a step.
pub enum SyncStatus {
    SyncedToLastBlock(StateSyncUpdate),
    SyncedToBlock(StateSyncUpdate),
}

impl SyncStatus {
    pub fn is_last_block(&self) -> bool {
        matches!(self, SyncStatus::SyncedToLastBlock(_))
    }
}

impl From<SyncStatus> for StateSyncUpdate {
    fn from(value: SyncStatus) -> StateSyncUpdate {
        match value {
            SyncStatus::SyncedToLastBlock(update) => update,
            SyncStatus::SyncedToBlock(update) => update,
        }
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
        CommittedNote,
        BlockHeader,
    ) -> Pin<Box<dyn Future<Output = Result<(NoteUpdates, Vec<NoteId>), ClientError>>>>,
>;

/// Callback to be executed when a transaction is marked committed in the sync response. It receives
/// the transaction update received from the node. It returns the note updates and transaction
/// updates that should be applied to the store as a result of the transaction being committed.
pub type OnTransactionCommitted = Box<
    dyn Fn(
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
        NullifierUpdate,
    )
        -> Pin<Box<dyn Future<Output = Result<(NoteUpdates, TransactionUpdates), ClientError>>>>,
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
    on_committed_transaction: OnTransactionCommitted,
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
    /// * `on_committed_transaction` - A callback to be executed when a transaction is committed.
    /// * `on_nullifier_received` - A callback to be executed when a nullifier is received.
    pub fn new(
        rpc_api: Arc<dyn NodeRpcClient + Send>,
        on_note_received: OnNoteReceived,
        on_committed_transaction: OnTransactionCommitted,
        on_nullifier_received: OnNullifierReceived,
    ) -> Self {
        Self {
            rpc_api,
            on_note_received,
            on_committed_transaction,
            on_nullifier_received,
        }
    }

    /// Executes a single step of the state sync process, returning the changes that should be
    /// applied to the store.
    ///
    /// A step in this context means a single request to the node to get the next relevant block and
    /// the changes that happened in it. This block may not be the last one in the chain and
    /// the client may need to call this method multiple times until it reaches the chain tip.
    /// Wheter or not the client has reached the chain tip is indicated by the returned
    /// [SyncStatus] variant. `None` is returned if the client is already synced with the chain tip
    /// and there are no new changes.
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
    pub async fn sync_state_step(
        &self,
        current_block: BlockHeader,
        current_block_has_relevant_notes: bool,
        current_partial_mmr: PartialMmr,
        accounts: Vec<AccountHeader>,
        note_tags: Vec<NoteTag>,
        unspent_nullifiers: Vec<Nullifier>,
    ) -> Result<Option<SyncStatus>, ClientError> {
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
            .sync_state(current_block_num, &account_ids, &note_tags, &nullifiers_tags)
            .await?;

        // We don't need to continue if the chain has not advanced, there are no new changes
        if response.block_header.block_num() == current_block_num {
            return Ok(None);
        }

        let account_updates =
            self.account_state_sync(&accounts, &response.account_hash_updates).await?;

        let (note_updates, transaction_updates, tags_to_remove) = self
            .note_state_sync(
                response.note_inclusions,
                response.transactions,
                response.nullifiers,
                response.block_header,
            )
            .await?;

        let (new_mmr_peaks, new_authentication_nodes) = apply_mmr_changes(
            current_block,
            current_block_has_relevant_notes,
            current_partial_mmr,
            response.mmr_delta,
        )
        .await?;

        let update = StateSyncUpdate {
            block_header: response.block_header,
            note_updates,
            transaction_updates,
            new_mmr_peaks,
            new_authentication_nodes,
            account_updates,
            tags_to_remove,
        };

        if response.chain_tip == response.block_header.block_num() {
            Ok(Some(SyncStatus::SyncedToLastBlock(update)))
        } else {
            Ok(Some(SyncStatus::SyncedToBlock(update)))
        }
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------
    /// Compares the state of tracked accounts with the updates received from the node and returns
    /// the accounts that need to be updated.
    ///
    /// When a mismatch is detected, two scenarios are possible:
    /// * If the account is public, the component will request the node for the updated account
    ///   details.
    /// * If the account is private it will be marked as mismatched and the client will need to
    ///   handle it (it could be a stale account state or a reason to lock the account).
    async fn account_state_sync(
        &self,
        accounts: &[AccountHeader],
        account_hash_updates: &[(AccountId, Digest)],
    ) -> Result<AccountUpdates, ClientError> {
        let (public_accounts, offchain_accounts): (Vec<_>, Vec<_>) =
            accounts.iter().partition(|account_header| account_header.id().is_public());

        let updated_public_accounts =
            self.get_updated_public_accounts(account_hash_updates, &public_accounts).await?;

        let mismatched_private_accounts = account_hash_updates
            .iter()
            .filter(|(account_id, digest)| {
                offchain_accounts
                    .iter()
                    .any(|account| account.id() == *account_id && &account.hash() != digest)
            })
            .cloned()
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
    /// by the client. It returns the updates that should be applied to the store.
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
        &self,
        note_inclusions: Vec<CommittedNote>,
        transactions: Vec<TransactionUpdate>,
        mut nullifiers: Vec<NullifierUpdate>,
        block_header: BlockHeader,
    ) -> Result<(NoteUpdates, TransactionUpdates, Vec<NoteTagRecord>), ClientError> {
        let mut note_updates = NoteUpdates::default();
        let mut public_note_ids = vec![];

        for committed_note in note_inclusions {
            let (new_note_update, new_note_ids) =
                (self.on_note_received)(committed_note, block_header).await?;
            note_updates.extend(new_note_update);
            public_note_ids.extend(new_note_ids);
        }

        let new_public_notes =
            self.fetch_public_note_details(&public_note_ids, &block_header).await?;

        note_updates.extend(NoteUpdates::new(new_public_notes, vec![], vec![], vec![]));

        // We can remove tags from notes that got committed
        let tags_to_remove = note_updates
            .updated_input_notes()
            .iter()
            .filter(|note| note.is_committed())
            .map(|note| {
                NoteTagRecord::with_note_source(
                    note.metadata().expect("Committed note should have metadata").tag(),
                    note.id(),
                )
            })
            .collect();

        let mut transaction_updates = TransactionUpdates::default();

        for transaction_update in transactions {
            let (new_note_update, new_transaction_update) =
                (self.on_committed_transaction)(transaction_update).await?;

            // Remove nullifiers if they were consumed by the transaction
            nullifiers.retain(|nullifier| {
                !new_note_update
                    .updated_input_notes()
                    .iter()
                    .any(|note| note.nullifier() == nullifier.nullifier)
            });

            note_updates.extend(new_note_update);
            transaction_updates.extend(new_transaction_update);
        }

        for nullifier_update in nullifiers {
            let (new_note_update, new_transaction_update) =
                (self.on_nullifier_received)(nullifier_update).await?;
            note_updates.extend(new_note_update);
            transaction_updates.extend(new_transaction_update);
        }

        Ok((note_updates, transaction_updates, tags_to_remove))
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

// DEFAULT CALLBACK IMPLEMENTATIONS
// ================================================================================================

/// Default implementation of the [OnNoteReceived] callback. It queries the store for the committed
/// note and updates the note records accordingly. If the note is not being tracked, it returns the
/// note ID to be queried from the node so it can be queried from the node and tracked.
pub async fn on_note_received(
    store: Arc<dyn Store>,
    committed_note: CommittedNote,
    block_header: BlockHeader,
) -> Result<(NoteUpdates, Vec<NoteId>), ClientError> {
    let inclusion_proof = NoteInclusionProof::new(
        block_header.block_num(),
        committed_note.note_index(),
        committed_note.merkle_path().clone(),
    )?;

    let mut updated_input_notes = vec![];
    let mut updated_output_notes = vec![];
    let mut new_note_ids = vec![];

    if let Some(mut input_note_record) = store
        .get_input_notes(NoteFilter::List(vec![*committed_note.note_id()]))
        .await?
        .pop()
    {
        // The note belongs to our locally tracked set of input notes
        let inclusion_proof_received = input_note_record
            .inclusion_proof_received(inclusion_proof.clone(), committed_note.metadata())?;
        let block_header_received = input_note_record.block_header_received(block_header)?;

        if inclusion_proof_received || block_header_received {
            updated_input_notes.push(input_note_record);
        }
    }

    if let Some(mut output_note_record) = store
        .get_output_notes(NoteFilter::List(vec![*committed_note.note_id()]))
        .await?
        .pop()
    {
        // The note belongs to our locally tracked set of output notes
        if output_note_record.inclusion_proof_received(inclusion_proof.clone())? {
            updated_output_notes.push(output_note_record);
        }
    }

    if updated_input_notes.is_empty() && updated_output_notes.is_empty() {
        // The note is public and we are not tracking it, push to the list of IDs to query
        new_note_ids.push(*committed_note.note_id());
    }

    Ok((
        NoteUpdates::new(vec![], vec![], updated_input_notes, updated_output_notes),
        new_note_ids,
    ))
}

/// Applies changes to the current MMR structure, returns the updated [MmrPeaks] and the
/// authentication nodes for leaves we track.
pub(crate) async fn apply_mmr_changes(
    current_block: BlockHeader,
    current_block_has_relevant_notes: bool,
    mut current_partial_mmr: PartialMmr,
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

/// Default implementation of the [OnTransactionCommitted] callback. It queries the store for the
/// input notes that were consumed by the transaction and updates the note records accordingly. It
/// also returns the committed transaction update to be applied to the store.
pub async fn on_transaction_committed(
    store: Arc<dyn Store>,
    transaction_update: TransactionUpdate,
) -> Result<(NoteUpdates, TransactionUpdates), ClientError> {
    // TODO: This could be improved if we add a filter to get only notes that are being processed by
    // a specific transaction
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

    let mut updated_input_notes = vec![];
    let mut updated_output_notes = vec![];
    for mut input_note_record in consumed_input_notes {
        if input_note_record.transaction_committed(
            transaction_update.transaction_id,
            transaction_update.block_num,
        )? {
            updated_input_notes.push(input_note_record);
        }
    }

    for mut output_note_record in consumed_output_notes {
        // SAFETY: Output notes were queried from a nullifier list and should have a nullifier
        let nullifier = output_note_record.nullifier().unwrap();
        if output_note_record.nullifier_received(nullifier, transaction_update.block_num)? {
            updated_output_notes.push(output_note_record);
        }
    }

    Ok((
        NoteUpdates::new(vec![], vec![], updated_input_notes, updated_output_notes),
        TransactionUpdates::new(vec![transaction_update], vec![]),
    ))
}

/// Default implementation of the [OnNullifierReceived] callback. It queries the store for the notes
/// that match the nullifier and updates the note records accordingly. It also returns the
/// transactions that should be discarded as they weren't committed when the nullifier was received.
pub async fn on_nullifier_received(
    store: Arc<dyn Store>,
    nullifier_update: NullifierUpdate,
) -> Result<(NoteUpdates, TransactionUpdates), ClientError> {
    let mut discarded_transactions = vec![];
    let mut updated_input_notes = vec![];
    let mut updated_output_notes = vec![];

    if let Some(mut input_note_record) = store
        .get_input_notes(NoteFilter::Nullifiers(vec![nullifier_update.nullifier]))
        .await?
        .pop()
    {
        if input_note_record.is_processing() {
            discarded_transactions.push(
                *input_note_record
                    .consumer_transaction_id()
                    .expect("Processing note should have consumer transaction id"),
            );
        }

        if input_note_record
            .consumed_externally(nullifier_update.nullifier, nullifier_update.block_num)?
        {
            updated_input_notes.push(input_note_record);
        }
    }

    if let Some(mut output_note_record) = store
        .get_output_notes(NoteFilter::Nullifiers(vec![nullifier_update.nullifier]))
        .await?
        .pop()
    {
        if output_note_record
            .nullifier_received(nullifier_update.nullifier, nullifier_update.block_num)?
        {
            updated_output_notes.push(output_note_record);
        }
    }

    Ok((
        NoteUpdates::new(vec![], vec![], updated_input_notes, updated_output_notes),
        TransactionUpdates::new(vec![], discarded_transactions),
    ))
}

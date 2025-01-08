use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{future::Future, pin::Pin};
use std::collections::BTreeMap;

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
    store::{InputNoteRecord, NoteFilter, OutputNoteRecord, Store, StoreError},
    transactions::TransactionUpdates,
    ClientError,
};

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
    pub fn into_state_sync_update(self) -> StateSyncUpdate {
        match self {
            SyncStatus::SyncedToLastBlock(update) => update,
            SyncStatus::SyncedToBlock(update) => update,
        }
    }
}

type NoteInclusionUpdate = Box<
    dyn Fn(
        Vec<CommittedNote>,
        BlockHeader,
    ) -> Pin<Box<dyn Future<Output = Result<NoteUpdates, ClientError>>>>,
>;

type NewNullifierUpdate = Box<
    dyn Fn(
        Vec<NullifierUpdate>,
        Vec<TransactionUpdate>,
    )
        -> Pin<Box<dyn Future<Output = Result<(NoteUpdates, TransactionUpdates), ClientError>>>>,
>;

type AccountHashUpdate = Box<
    dyn Fn(
        Vec<(AccountId, Digest)>,
    ) -> Pin<Box<dyn Future<Output = Result<AccountUpdates, ClientError>>>>,
>;

/// The state sync components encompasses the client's sync logic.
///
/// When created it receives the current state of the client's relevant elements (block, accounts,
/// notes, etc). It is then used to requset updates from the node and apply them to the relevant
/// elements. The updates are then returned and can be applied to the store to persist the changes.
pub struct StateSync {
    rpc_api: Arc<dyn NodeRpcClient + Send>,
    note_inclusion_update: NoteInclusionUpdate,
    new_nullifier_update: NewNullifierUpdate,
    account_hash_update: AccountHashUpdate,
}

impl StateSync {
    /// Creates a new instance of the state sync component.
    ///
    /// # Arguments
    ///
    /// * `rpc_api` - The RPC client to use to communicate with the node.
    /// * `current_block` - The latest block header tracked by the client.
    /// * `current_block_has_relevant_notes` - A flag indicating if the current block has notes that
    ///   are relevant to the client.
    /// * `accounts` - The headers of accounts tracked by the client.
    /// * `note_tags` - The note tags to be used in the sync state request.
    /// * `unspent_input_notes` - The input notes that haven't been yet consumed and may be changed
    ///   in the sync process.
    /// * `unspent_output_notes` - The output notes that haven't been yet consumed and may be
    ///   changed in the sync process.
    /// * `current_partial_mmr` - The current partial MMR of the client.
    pub fn new(
        rpc_api: Arc<dyn NodeRpcClient + Send>,
        note_inclusion_update: NoteInclusionUpdate,
        new_nullifier_update: NewNullifierUpdate,
        account_hash_update: AccountHashUpdate,
    ) -> Self {
        Self {
            rpc_api,
            note_inclusion_update,
            new_nullifier_update,
            account_hash_update,
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
    pub async fn sync_state_step(
        &self,
        current_block: BlockHeader,
        current_block_has_relevant_notes: bool,
        current_partial_mmr: PartialMmr,
        account_ids: Vec<AccountId>,
        note_tags: Vec<NoteTag>,
        unspent_nullifiers: Vec<Nullifier>,
    ) -> Result<Option<SyncStatus>, ClientError> {
        let current_block_num = current_block.block_num();

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

        let account_updates = (self.account_hash_update)(response.account_hash_updates).await?;

        let committed_note_updates =
            (self.note_inclusion_update)(response.note_inclusions, response.block_header).await?;

        let (consumed_note_updates, transaction_updates) =
            (self.new_nullifier_update)(response.nullifiers, response.transactions).await?;

        // We can remove tags from notes that got committed
        let tags_to_remove = committed_note_updates
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

        let (new_mmr_peaks, new_authentication_nodes) = self
            .apply_mmr_changes(
                current_block,
                current_block_has_relevant_notes,
                current_partial_mmr,
                response.mmr_delta,
            )
            .await?;

        let update = StateSyncUpdate {
            block_header: response.block_header,
            note_updates: committed_note_updates.combine_with(consumed_note_updates),
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

    /// Applies changes to the current MMR structure, returns the updated [MmrPeaks] and the
    /// authentication nodes for leaves we track.
    pub(crate) async fn apply_mmr_changes(
        &self,
        current_block: BlockHeader,
        current_block_has_relevant_notes: bool,
        mut current_partial_mmr: PartialMmr,
        mmr_delta: MmrDelta,
    ) -> Result<(MmrPeaks, Vec<(InOrderIndex, Digest)>), ClientError> {
        // First, apply curent_block to the Mmr
        let new_authentication_nodes = current_partial_mmr
            .add(current_block.hash(), current_block_has_relevant_notes)
            .into_iter();

        // Apply the Mmr delta to bring Mmr to forest equal to chain tip
        let new_authentication_nodes: Vec<(InOrderIndex, Digest)> = current_partial_mmr
            .apply(mmr_delta)
            .map_err(StoreError::MmrError)?
            .into_iter()
            .chain(new_authentication_nodes)
            .collect();

        Ok((current_partial_mmr.peaks(), new_authentication_nodes))
    }
}

/// Returns the [NoteUpdates] containing new public note and committed input/output notes and a
/// list or note tag records to be removed from the store.
pub async fn committed_note_updates(
    store: Arc<dyn Store>,
    rpc_api: Arc<dyn NodeRpcClient + Send>,
    committed_notes: Vec<CommittedNote>,
    block_header: BlockHeader,
) -> Result<NoteUpdates, ClientError> {
    // We'll only pick committed notes that we are tracking as input/output notes. Since the
    // sync response contains notes matching either the provided accounts or the provided tag
    // we might get many notes when we only care about a few of those.
    let relevant_note_filter =
        NoteFilter::List(committed_notes.iter().map(|note| note.note_id()).cloned().collect());

    let mut committed_input_notes: BTreeMap<NoteId, InputNoteRecord> = store
        .get_input_notes(relevant_note_filter.clone())
        .await?
        .into_iter()
        .map(|n| (n.id(), n))
        .collect();

    let mut committed_output_notes: BTreeMap<NoteId, OutputNoteRecord> = store
        .get_output_notes(relevant_note_filter)
        .await?
        .into_iter()
        .map(|n| (n.id(), n))
        .collect();

    let mut new_public_notes = vec![];
    let mut committed_tracked_input_notes = vec![];
    let mut committed_tracked_output_notes = vec![];

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
        fetch_public_note_details(store, rpc_api, &new_public_notes, &block_header).await?;

    Ok(NoteUpdates::new(
        new_public_notes,
        vec![],
        committed_tracked_input_notes,
        committed_tracked_output_notes,
    ))
}

/// Queries the node for all received notes that aren't being locally tracked in the client.
///
/// The client can receive metadata for private notes that it's not tracking. In this case,
/// notes are ignored for now as they become useless until details are imported.
async fn fetch_public_note_details(
    store: Arc<dyn Store>,
    rpc_api: Arc<dyn NodeRpcClient + Send>,
    query_notes: &[NoteId],
    block_header: &BlockHeader,
) -> Result<Vec<InputNoteRecord>, ClientError> {
    if query_notes.is_empty() {
        return Ok(vec![]);
    }
    info!("Getting note details for notes that are not being tracked.");

    let mut return_notes = rpc_api
        .get_public_note_records(query_notes, store.get_current_timestamp())
        .await?;

    for note in return_notes.iter_mut() {
        note.block_header_received(*block_header)?;
    }

    Ok(return_notes)
}

/// Returns the [NoteUpdates] containing consumed input/output notes and a list of IDs of the
/// transactions that were discarded.
pub async fn consumed_note_updates(
    store: Arc<dyn Store>,
    nullifiers: Vec<NullifierUpdate>,
    committed_transactions: Vec<TransactionUpdate>,
) -> Result<(NoteUpdates, TransactionUpdates), ClientError> {
    let nullifier_filter = NoteFilter::Nullifiers(
        nullifiers.iter().map(|nullifier_update| nullifier_update.nullifier).collect(),
    );

    let mut consumed_input_notes: BTreeMap<Nullifier, InputNoteRecord> = store
        .get_input_notes(nullifier_filter.clone())
        .await?
        .into_iter()
        .map(|n| (n.nullifier(), n))
        .collect();

    let mut consumed_output_notes: BTreeMap<Nullifier, OutputNoteRecord> = store
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
    for transaction_update in committed_transactions.iter() {
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
        TransactionUpdates::new(committed_transactions, discarded_transactions),
    ))
}

/// Compares the state of tracked accounts with the updates received from the node and returns
/// the accounts that need to be updated.
///
/// When a mismatch is detected, two scenarios are possible:
/// * If the account is public, the component will request the node for the updated account details.
/// * If the account is private it will be marked as mismatched and the client will need to handle
///   it (it could be a stale account state or a reason to lock the account).
pub async fn account_state_sync(
    store: Arc<dyn Store>,
    rpc_api: Arc<dyn NodeRpcClient + Send>,
    account_hash_updates: Vec<(AccountId, Digest)>,
) -> Result<AccountUpdates, ClientError> {
    let (public_accounts, private_accounts): (Vec<_>, Vec<_>) = store
        .get_account_headers()
        .await?
        .into_iter()
        .map(|(acc, _)| acc)
        .partition(|acc| acc.id().is_public());

    let updated_public_accounts =
        get_updated_public_accounts(rpc_api, &account_hash_updates, &public_accounts).await?;

    let mismatched_private_accounts = account_hash_updates
        .iter()
        .filter(|(new_id, new_hash)| {
            private_accounts
                .iter()
                .any(|acc| acc.id() == *new_id && acc.hash() != *new_hash)
        })
        .cloned()
        .collect::<Vec<_>>();

    Ok(AccountUpdates::new(updated_public_accounts, mismatched_private_accounts))
}

async fn get_updated_public_accounts(
    rpc_api: Arc<dyn NodeRpcClient + Send>,
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

    rpc_api
        .get_updated_public_accounts(&mismatched_public_accounts)
        .await
        .map_err(ClientError::RpcError)
}

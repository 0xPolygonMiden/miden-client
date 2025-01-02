use alloc::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    vec::Vec,
};

use miden_objects::{
    accounts::{Account, AccountHeader, AccountId},
    crypto::merkle::{InOrderIndex, MmrDelta, MmrPeaks, PartialMmr},
    notes::{NoteId, NoteInclusionProof, NoteTag, Nullifier},
    transaction::{InputNote, TransactionId},
    BlockHeader, Digest,
};
use tracing::info;

use super::{get_nullifier_prefix, NoteTagRecord, SyncSummary};
use crate::{
    accounts::AccountUpdates,
    notes::NoteUpdates,
    rpc::{
        domain::{
            accounts::AccountDetails,
            notes::{CommittedNote, NoteDetails},
            nullifiers::NullifierUpdate,
            transactions::TransactionUpdate,
        },
        NodeRpcClient, RpcError,
    },
    store::{InputNoteRecord, OutputNoteRecord, StoreError},
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

/// The state sync components encompasses the client's sync logic.
///
/// When created it receives the current state of the client's relevant elements (block, accounts,
/// notes, etc). It is then used to requset updates from the node and apply them to the relevant
/// elements. The updates are then returned and can be applied to the store to persist the changes.
pub struct StateSync {
    rpc_api: Arc<dyn NodeRpcClient + Send>,
    current_block: BlockHeader,
    current_block_has_relevant_notes: bool,
    accounts: Vec<AccountHeader>,
    note_tags: Vec<NoteTag>,
    unspent_input_notes: BTreeMap<NoteId, InputNoteRecord>,
    unspent_output_notes: BTreeMap<NoteId, OutputNoteRecord>,
    changed_notes: BTreeSet<NoteId>,
    current_partial_mmr: PartialMmr,
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
        current_block: BlockHeader,
        current_block_has_relevant_notes: bool,
        accounts: Vec<AccountHeader>,
        note_tags: Vec<NoteTag>,
        unspent_input_notes: Vec<InputNoteRecord>,
        unspent_output_notes: Vec<OutputNoteRecord>,
        current_partial_mmr: PartialMmr,
    ) -> Self {
        let unspent_input_notes =
            unspent_input_notes.into_iter().map(|note| (note.id(), note)).collect();
        let unspent_output_notes =
            unspent_output_notes.into_iter().map(|note| (note.id(), note)).collect();

        Self {
            rpc_api,
            current_block,
            current_block_has_relevant_notes,
            accounts,
            note_tags,
            unspent_input_notes,
            unspent_output_notes,
            changed_notes: BTreeSet::new(),
            current_partial_mmr,
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
    pub async fn sync_state_step(mut self) -> Result<Option<SyncStatus>, ClientError> {
        let current_block_num = self.current_block.block_num();
        let account_ids: Vec<AccountId> = self.accounts.iter().map(|acc| acc.id()).collect();

        // To receive information about added nullifiers, we reduce them to the higher 16 bits
        // Note that besides filtering by nullifier prefixes, the node also filters by block number
        // (it only returns nullifiers from current_block_num until
        // response.block_header.block_num())
        let input_note_nullifiers = self
            .unspent_input_notes
            .values()
            .map(|note| get_nullifier_prefix(&note.nullifier()));

        let output_note_nullifiers = self
            .unspent_output_notes
            .values()
            .filter_map(|note| note.nullifier())
            .map(|nullifier| get_nullifier_prefix(&nullifier));

        let nullifiers_tags: Vec<u16> =
            input_note_nullifiers.chain(output_note_nullifiers).collect();

        let response = self
            .rpc_api
            .sync_state(current_block_num, &account_ids, &self.note_tags, &nullifiers_tags)
            .await?;

        // We don't need to continue if the chain has not advanced, there are no new changes
        if response.block_header.block_num() == current_block_num {
            return Ok(None);
        }

        let (note_updates, transaction_updates) = self
            .note_state_sync(
                &response.block_header,
                response.note_inclusions,
                response.nullifiers,
                response.transactions,
            )
            .await?;

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

        // ACCOUNTS
        let account_updates = self.account_state_sync(&response.account_hash_updates).await?;

        // MMR
        let new_authentication_nodes = self.update_partial_mmr(response.mmr_delta).await?;

        let update = StateSyncUpdate {
            block_header: response.block_header,
            note_updates,
            transaction_updates,
            new_mmr_peaks: self.current_partial_mmr.peaks(),
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
    ///  handle it (it could be a stale account state or a reason to lock the account).
    async fn account_state_sync(
        &self,
        account_hash_updates: &[(AccountId, Digest)],
    ) -> Result<AccountUpdates, ClientError> {
        let (onchain_accounts, offchain_accounts): (Vec<_>, Vec<_>) =
            self.accounts.iter().partition(|account_header| account_header.id().is_public());

        let updated_onchain_accounts = self
            .get_updated_onchain_accounts(account_hash_updates, &onchain_accounts)
            .await?;

        let mismatched_private_accounts = account_hash_updates
            .iter()
            .filter(|(account_id, digest)| {
                offchain_accounts
                    .iter()
                    .any(|account| account.id() == *account_id && &account.hash() != digest)
            })
            .cloned()
            .collect::<Vec<_>>();

        Ok(AccountUpdates::new(updated_onchain_accounts, mismatched_private_accounts))
    }

    /// Compares the state of tracked notes with the updates received from the node and returns the
    /// note and transaction changes that should be applied to the store.
    ///
    /// The note changes might include:
    /// * New notes that we received from the node and might be relevant to the client.
    /// * Tracked expected notes that were committed in the block.
    /// * Tracked notes that were being processed by a transaction that got committed.
    /// * Tracked notes that were nullified by an external transaction.
    ///
    /// The transaction changes might include:
    /// * Transactions that were committed in the block. Some of these might me tracked by the
    ///   client
    ///  and need to be marked as committed.
    /// * Local tracked transactions that were discarded because the notes that they were processing
    /// were nullified by an another transaction.
    async fn note_state_sync(
        &mut self,
        block_header: &BlockHeader,
        committed_notes: Vec<CommittedNote>,
        nullifiers: Vec<NullifierUpdate>,
        committed_transactions: Vec<TransactionUpdate>,
    ) -> Result<(NoteUpdates, TransactionUpdates), ClientError> {
        let new_notes = self.committed_note_updates(committed_notes, block_header).await?;

        let discarded_transactions =
            self.consumed_note_updates(nullifiers, &committed_transactions).await?;

        let modified_input_notes: Vec<InputNoteRecord> = self
            .changed_notes
            .iter()
            .filter_map(|note_id| self.unspent_input_notes.remove(note_id))
            .collect();

        let modified_output_notes: Vec<OutputNoteRecord> = self
            .changed_notes
            .iter()
            .filter_map(|note_id| self.unspent_output_notes.remove(note_id))
            .collect();

        let note_updates = NoteUpdates::new(new_notes, modified_input_notes, modified_output_notes);
        let transaction_updates =
            TransactionUpdates::new(committed_transactions, discarded_transactions);

        Ok((note_updates, transaction_updates))
    }

    /// Updates the unspent notes with the notes that were committed in the block. Returns the IDs
    /// of new public notes that matched the provided tags.
    async fn committed_note_updates(
        &mut self,
        committed_notes: Vec<CommittedNote>,
        block_header: &BlockHeader,
    ) -> Result<Vec<InputNote>, ClientError> {
        let mut new_public_notes = vec![];

        // We'll only pick committed notes that we are tracking as input/output notes. Since the
        // sync response contains notes matching either the provided accounts or the provided tag
        // we might get many notes when we only care about a few of those.
        for committed_note in committed_notes {
            let inclusion_proof = NoteInclusionProof::new(
                block_header.block_num(),
                committed_note.note_index(),
                committed_note.merkle_path().clone(),
            )?;

            if let Some(note_record) = self.unspent_input_notes.get_mut(committed_note.note_id()) {
                // The note belongs to our locally tracked set of input notes
                let inclusion_proof_received = note_record
                    .inclusion_proof_received(inclusion_proof.clone(), committed_note.metadata())?;
                let block_header_received = note_record.block_header_received(*block_header)?;

                if inclusion_proof_received || block_header_received {
                    self.changed_notes.insert(*committed_note.note_id());
                }
            }

            if let Some(note_record) = self.unspent_output_notes.get_mut(committed_note.note_id()) {
                // The note belongs to our locally tracked set of output notes
                if note_record.inclusion_proof_received(inclusion_proof.clone())? {
                    self.changed_notes.insert(*committed_note.note_id());
                }
            }

            if !self.unspent_input_notes.contains_key(committed_note.note_id())
                && !self.unspent_output_notes.contains_key(committed_note.note_id())
            {
                // The note totally new to the client
                new_public_notes.push(*committed_note.note_id());
            }
        }

        // Query the node for input note data and build the entities
        let new_public_notes =
            self.fetch_public_note_details(&new_public_notes, block_header).await?;

        Ok(new_public_notes)
    }

    /// Updates the unspent notes to nullify those that were consumed (either internally or
    /// externally). Returns the IDs of the transactions that were discarded.
    async fn consumed_note_updates(
        &mut self,
        nullifiers: Vec<NullifierUpdate>,
        committed_transactions: &[TransactionUpdate],
    ) -> Result<Vec<TransactionId>, ClientError> {
        let nullifier_filter: Vec<Nullifier> = nullifiers.iter().map(|n| n.nullifier).collect();

        let consumed_input_notes = self
            .unspent_input_notes
            .values()
            .filter(|&n| nullifier_filter.contains(&n.nullifier()))
            .map(|n| (n.nullifier(), n.id()));

        let consumed_output_notes = self
            .unspent_output_notes
            .values()
            .filter(|&n| n.nullifier().is_some_and(|n| nullifier_filter.contains(&n)))
            .map(|n| {
                (n.nullifier().expect("Output notes without nullifier were filtered"), n.id())
            });

        let mut consumed_note_ids: BTreeMap<Nullifier, NoteId> =
            consumed_input_notes.chain(consumed_output_notes).collect();

        // Modify notes that were being processed by a transaciton that just got committed. These
        // notes were consumed internally.
        for transaction_update in committed_transactions {
            // Get the notes that were being processed by the transaction
            let transaction_consumed_notes: Vec<NoteId> = consumed_note_ids
                .iter()
                .filter_map(|(_, note_id)| {
                    let note_record = self.unspent_input_notes.get(note_id)?;
                    if note_record.is_processing()
                        && note_record.consumer_transaction_id()
                            == Some(&transaction_update.transaction_id)
                    {
                        Some(note_id)
                    } else {
                        None
                    }
                })
                .cloned()
                .collect();

            for note_id in transaction_consumed_notes {
                if let Some(input_note_record) = self.unspent_input_notes.get_mut(&note_id) {
                    if input_note_record.transaction_committed(
                        transaction_update.transaction_id,
                        transaction_update.block_num,
                    )? {
                        self.changed_notes.insert(note_id);
                    }
                }
            }
        }

        let mut discarded_transactions = vec![];
        // Modify notes that were nullified and didn't have a committed transaction to consume them
        // in the previous step. These notes were consumed externally.
        for nullifier_update in nullifiers {
            let nullifier = nullifier_update.nullifier;
            let block_num = nullifier_update.block_num;

            if let Some(note_id) = consumed_note_ids.remove(&nullifier) {
                if let Some(input_note_record) = self.unspent_input_notes.get_mut(&note_id) {
                    if input_note_record.is_processing() {
                        // The input note was being processed by a local transaction but it was
                        // nullified externally so the transaction should be
                        // discarded
                        discarded_transactions.push(
                            *input_note_record
                                .consumer_transaction_id()
                                .expect("Processing note should have consumer transaction id"),
                        );
                    }

                    if input_note_record.consumed_externally(nullifier, block_num)? {
                        self.changed_notes.insert(note_id);
                    }
                }

                if let Some(output_note_record) = self.unspent_output_notes.get_mut(&note_id) {
                    if output_note_record.nullifier_received(nullifier, block_num)? {
                        self.changed_notes.insert(note_id);
                    }
                }
            }
        }

        Ok(discarded_transactions)
    }

    /// Queries the node for all received notes that aren't being locally tracked in the client.
    ///
    /// The client can receive metadata for private notes that it's not tracking. In this case,
    /// notes are ignored for now as they become useless until details are imported.
    async fn fetch_public_note_details(
        &self,
        query_notes: &[NoteId],
        block_header: &BlockHeader,
    ) -> Result<Vec<InputNote>, ClientError> {
        if query_notes.is_empty() {
            return Ok(vec![]);
        }
        info!("Getting note details for notes that are not being tracked.");

        let notes_data = self.rpc_api.get_notes_by_id(query_notes).await?;
        let mut return_notes = Vec::with_capacity(query_notes.len());
        for note_data in notes_data {
            match note_data {
                NoteDetails::Private(id, ..) => {
                    // TODO: Is there any benefit to not ignoring these? In any case we do not have
                    // the recipient which is mandatory right now.
                    info!("Note {} is private but the client is not tracking it, ignoring.", id);
                },
                NoteDetails::Public(note, inclusion_proof) => {
                    info!("Retrieved details for Note ID {}.", note.id());
                    let inclusion_proof = NoteInclusionProof::new(
                        block_header.block_num(),
                        inclusion_proof.note_index,
                        inclusion_proof.merkle_path,
                    )
                    .map_err(ClientError::NoteError)?;

                    return_notes.push(InputNote::authenticated(note, inclusion_proof))
                },
            }
        }
        Ok(return_notes)
    }

    async fn get_updated_onchain_accounts(
        &self,
        account_updates: &[(AccountId, Digest)],
        current_onchain_accounts: &[&AccountHeader],
    ) -> Result<Vec<Account>, ClientError> {
        let mut accounts_to_update: Vec<Account> = Vec::new();
        for (remote_account_id, remote_account_hash) in account_updates {
            // check if this updated account is tracked by the client
            let current_account = current_onchain_accounts
                .iter()
                .find(|acc| *remote_account_id == acc.id() && *remote_account_hash != acc.hash());

            if let Some(tracked_account) = current_account {
                info!("Public account hash difference detected for account with ID: {}. Fetching node for updates...", tracked_account.id());
                let account_details = self.rpc_api.get_account_update(tracked_account.id()).await?;
                if let AccountDetails::Public(account, _) = account_details {
                    // We should only do the update if it's newer, otherwise we ignore it
                    if account.nonce().as_int() > tracked_account.nonce().as_int() {
                        accounts_to_update.push(account);
                    }
                } else {
                    return Err(RpcError::AccountUpdateForPrivateAccountReceived(
                        account_details.account_id(),
                    )
                    .into());
                }
            }
        }
        Ok(accounts_to_update)
    }

    /// Updates the `current_partial_mmr` and returns the authentication nodes for tracked leaves.
    pub(crate) async fn update_partial_mmr(
        &mut self,
        mmr_delta: MmrDelta,
    ) -> Result<Vec<(InOrderIndex, Digest)>, ClientError> {
        // First, apply curent_block to the Mmr
        let new_authentication_nodes = self
            .current_partial_mmr
            .add(self.current_block.hash(), self.current_block_has_relevant_notes)
            .into_iter();

        // Apply the Mmr delta to bring Mmr to forest equal to chain tip
        let new_authentication_nodes: Vec<(InOrderIndex, Digest)> = self
            .current_partial_mmr
            .apply(mmr_delta)
            .map_err(StoreError::MmrError)?
            .into_iter()
            .chain(new_authentication_nodes)
            .collect();

        Ok(new_authentication_nodes)
    }
}

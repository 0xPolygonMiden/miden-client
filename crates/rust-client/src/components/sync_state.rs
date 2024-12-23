use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};

use miden_objects::{
    accounts::{Account, AccountHeader, AccountId},
    crypto::merkle::{InOrderIndex, MmrDelta, MmrPeaks, PartialMmr},
    notes::{NoteId, NoteInclusionProof, NoteTag, Nullifier},
    transaction::TransactionId,
    BlockHeader, Digest,
};
use tracing::*;

use crate::{
    accounts::AccountUpdates,
    notes::{NoteScreener, NoteUpdates},
    rpc::{
        domain::{
            accounts::AccountDetails,
            notes::{CommittedNote, NoteDetails},
            nullifiers::NullifierUpdate,
            transactions::TransactionUpdate,
        },
        NodeRpcClient, RpcError,
    },
    store::{
        input_note_states::CommittedNoteState, InputNoteRecord, NoteFilter, OutputNoteRecord,
        Store, StoreError, TransactionFilter,
    },
    sync::{get_nullifier_prefix, NoteTagRecord, SyncSummary},
    ClientError,
};

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

impl From<&StateSyncUpdate> for SyncSummary {
    fn from(value: &StateSyncUpdate) -> Self {
        SyncSummary::new(
            value.block_header.block_num(),
            value.note_updates.new_input_notes().iter().map(|n| n.id()).collect(),
            value.note_updates.committed_note_ids().into_iter().collect(),
            value.note_updates.consumed_note_ids().into_iter().collect(),
            value
                .updated_accounts
                .updated_onchain_accounts()
                .iter()
                .map(|acc| acc.id())
                .collect(),
            value
                .updated_accounts
                .mismatched_offchain_accounts()
                .iter()
                .map(|(acc_id, _)| *acc_id)
                .collect(),
            value.transactions_to_commit.iter().map(|tx| tx.transaction_id).collect(),
        )
    }
}

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

pub struct SyncState {
    /// The client's store, which provides a way to write and read entities to provide persistence.
    store: Arc<dyn Store>,
    /// An instance of [NodeRpcClient] which provides a way for the component to connect to the
    /// Miden node.
    rpc_api: Arc<dyn NodeRpcClient + Send>,
}

impl SyncState {
    /// Creates a new instance of [SyncState].
    pub fn new(store: Arc<dyn Store>, rpc_api: Arc<dyn NodeRpcClient + Send>) -> Self {
        Self { store, rpc_api }
    }

    pub async fn step_sync_state(
        &mut self,
        current_block_num: u32,
        tracked_accounts: Vec<AccountHeader>,
        note_tags: &[NoteTag],
        nullifiers: &[Nullifier],
    ) -> Result<Option<SyncStatus>, ClientError> {
        let account_ids: Vec<AccountId> =
            tracked_accounts.iter().map(|account| account.id()).collect();

        // To receive information about added nullifiers, we reduce them to the higher 16 bits
        // Note that besides filtering by nullifier prefixes, the node also filters by block number
        // (it only returns nullifiers from current_block_num until
        // response.block_header.block_num())
        let nullifier_tags: Vec<u16> = nullifiers.iter().map(get_nullifier_prefix).collect();

        let response = self
            .rpc_api
            .sync_state(current_block_num, &account_ids, note_tags, &nullifier_tags)
            .await?;

        // We don't need to continue if the chain has not advanced, there are no new changes
        if response.block_header.block_num() == current_block_num {
            return Ok(None);
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

        let (onchain_accounts, offchain_accounts): (Vec<_>, Vec<_>) = tracked_accounts
            .into_iter()
            .partition(|account_header| account_header.id().is_public());

        let updated_onchain_accounts = self
            .get_updated_onchain_accounts(&response.account_hash_updates, &onchain_accounts)
            .await?;

        let mismatched_offchain_accounts = self
            .validate_local_account_hashes(&response.account_hash_updates, &offchain_accounts)
            .await?;

        // Build PartialMmr with current data and apply updates
        let (new_peaks, new_authentication_nodes) = {
            let current_partial_mmr = self.store.build_current_partial_mmr(false).await?;

            let (current_block, has_relevant_notes) =
                self.store.get_block_header_by_num(current_block_num).await?;

            apply_mmr_changes(
                current_partial_mmr,
                response.mmr_delta,
                current_block,
                has_relevant_notes,
            )?
        };

        let sync_update = StateSyncUpdate {
            block_header: response.block_header,
            note_updates,
            transactions_to_commit,
            new_mmr_peaks: new_peaks,
            new_authentication_nodes,
            updated_accounts: AccountUpdates::new(
                updated_onchain_accounts,
                mismatched_offchain_accounts,
            ),
            block_has_relevant_notes: incoming_block_has_relevant_notes,
            transactions_to_discard,
            tags_to_remove,
        };

        if response.chain_tip == response.block_header.block_num() {
            Ok(Some(SyncStatus::SyncedToLastBlock(sync_update)))
        } else {
            Ok(Some(SyncStatus::SyncedToBlock(sync_update)))
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

    /// Queries the node for all received notes that are not being locally tracked in the client
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
                    let metadata = *note.metadata();

                    return_notes.push(InputNoteRecord::new(
                        note.into(),
                        self.store.get_current_timestamp(),
                        CommittedNoteState {
                            metadata,
                            inclusion_proof,
                            block_note_root: block_header.note_root(),
                        }
                        .into(),
                    ))
                },
            }
        }
        Ok(return_notes)
    }

    /// Extracts information about transactions for uncommitted transactions that the client is
    /// tracking from the received [SyncStateResponse]
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

    async fn get_updated_onchain_accounts(
        &mut self,
        account_updates: &[(AccountId, Digest)],
        current_onchain_accounts: &[AccountHeader],
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

    /// Validates account hash updates and returns a vector with all the offchain account
    /// mismatches.
    ///
    /// Offchain account mismatches happen when the hash account of the local tracked account
    /// doesn't match the hash account of the account in the node. This would be an anomaly and may
    /// happen for two main reasons:
    /// - A different client made a transaction with the account, changing its state.
    /// - The local transaction that modified the local state didn't go through, rendering the local
    ///   account state outdated.
    async fn validate_local_account_hashes(
        &mut self,
        account_updates: &[(AccountId, Digest)],
        current_offchain_accounts: &[AccountHeader],
    ) -> Result<Vec<(AccountId, Digest)>, ClientError> {
        let mut mismatched_accounts = vec![];

        for (remote_account_id, remote_account_hash) in account_updates {
            // ensure that if we track that account, it has the same hash
            let mismatched_account = current_offchain_accounts
                .iter()
                .find(|acc| *remote_account_id == acc.id() && *remote_account_hash != acc.hash());

            // OffChain accounts should always have the latest known state. If we receive a stale
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

    /// Checks the relevance of the block by verifying if any of the input notes in the block are
    /// relevant to the client. If any of the notes are relevant, the function returns `true`.
    pub(crate) async fn check_block_relevance(
        &mut self,
        committed_notes: &NoteUpdates,
    ) -> Result<bool, ClientError> {
        // We'll only do the check for either incoming public notes or expected input notes as
        // output notes are not really candidates to be consumed here.

        let note_screener = NoteScreener::new(self.store.clone());

        // Find all relevant Input Notes using the note checker
        for input_note in committed_notes
            .updated_input_notes()
            .iter()
            .chain(committed_notes.new_input_notes().iter())
        {
            if !note_screener
                .check_relevance(&input_note.try_into().map_err(ClientError::NoteRecordError)?)
                .await?
                .is_empty()
            {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

/// Applies changes to the Mmr structure, storing authentication nodes for leaves we track
/// and returns the updated [PartialMmr].
pub(crate) fn apply_mmr_changes(
    current_partial_mmr: PartialMmr,
    mmr_delta: MmrDelta,
    current_block_header: BlockHeader,
    current_block_has_relevant_notes: bool,
) -> Result<(MmrPeaks, Vec<(InOrderIndex, Digest)>), StoreError> {
    let mut partial_mmr: PartialMmr = current_partial_mmr;

    // First, apply curent_block to the Mmr
    let new_authentication_nodes = partial_mmr
        .add(current_block_header.hash(), current_block_has_relevant_notes)
        .into_iter();

    // Apply the Mmr delta to bring Mmr to forest equal to chain tip
    let new_authentication_nodes: Vec<(InOrderIndex, Digest)> = partial_mmr
        .apply(mmr_delta)
        .map_err(StoreError::MmrError)?
        .into_iter()
        .chain(new_authentication_nodes)
        .collect();

    Ok((partial_mmr.peaks(), new_authentication_nodes))
}

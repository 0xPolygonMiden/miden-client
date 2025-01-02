use alloc::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    vec::Vec,
};

use miden_objects::{
    accounts::{Account, AccountHeader, AccountId},
    crypto::merkle::MmrDelta,
    notes::{NoteId, NoteInclusionProof, NoteTag, Nullifier},
    transaction::TransactionId,
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
    store::{input_note_states::CommittedNoteState, InputNoteRecord},
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
    /// Information to update the local partial MMR.
    pub mmr_delta: MmrDelta,
    /// Information abount account changes after the sync.
    pub account_updates: AccountUpdates,
    /// Tag records that are no longer relevant.
    pub tags_to_remove: Vec<NoteTagRecord>,
}

impl StateSyncUpdate {
    pub fn new_empty(block_header: BlockHeader) -> Self {
        Self {
            block_header,
            note_updates: NoteUpdates::new(vec![], vec![], vec![], vec![]),
            transaction_updates: TransactionUpdates::new(vec![], vec![]),
            mmr_delta: MmrDelta { forest: 0, data: Vec::new() },
            account_updates: AccountUpdates::new(vec![], vec![]),
            tags_to_remove: vec![],
        }
    }
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
            vec![], // TODO add these fields
            vec![],
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

pub struct StateSync {
    rpc_api: Arc<dyn NodeRpcClient + Send>,
    current_block: BlockHeader,
    accounts: Vec<AccountHeader>,
    note_tags: Vec<NoteTag>,
    unspent_notes: BTreeMap<NoteId, InputNoteRecord>,
    changed_notes: BTreeSet<NoteId>,
}

impl StateSync {
    pub fn new(
        rpc_api: Arc<dyn NodeRpcClient + Send>,
        current_block: BlockHeader,
        accounts: Vec<AccountHeader>,
        note_tags: Vec<NoteTag>,
        unspent_notes: Vec<InputNoteRecord>,
    ) -> Self {
        let unspent_notes = unspent_notes.into_iter().map(|note| (note.id(), note)).collect();

        Self {
            rpc_api,
            current_block,
            accounts,
            note_tags,
            unspent_notes,
            changed_notes: BTreeSet::new(),
        }
    }

    pub async fn sync_state_step(mut self) -> Result<Option<SyncStatus>, ClientError> {
        let current_block_num = self.current_block.block_num();
        let account_ids: Vec<AccountId> = self.accounts.iter().map(|acc| acc.id()).collect();

        // To receive information about added nullifiers, we reduce them to the higher 16 bits
        // Note that besides filtering by nullifier prefixes, the node also filters by block number
        // (it only returns nullifiers from current_block_num until
        // response.block_header.block_num())
        let nullifiers_tags: Vec<u16> = self
            .unspent_notes
            .values()
            .map(|note| get_nullifier_prefix(&note.nullifier()))
            .collect();

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
            .filter_map(|note| {
                note.is_committed().then(|| {
                    NoteTagRecord::with_note_source(
                        note.metadata().expect("Committed note should have metadata").tag(),
                        note.id(),
                    )
                })
            })
            .collect();

        // ACCOUNTS
        let account_updates = self.account_state_sync(&response.account_hash_updates).await?;

        let update = StateSyncUpdate {
            block_header: response.block_header,
            note_updates,
            transaction_updates,
            mmr_delta: response.mmr_delta,
            account_updates,
            tags_to_remove, /* TODO: I think this can be removed from the update and be inferred
                             * from the note updates */
        };

        if response.chain_tip == response.block_header.block_num() {
            Ok(Some(SyncStatus::SyncedToLastBlock(update)))
        } else {
            Ok(Some(SyncStatus::SyncedToBlock(update)))
        }
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    async fn account_state_sync(
        &self,
        account_hash_updates: &[(AccountId, Digest)],
    ) -> Result<AccountUpdates, ClientError> {
        let (onchain_accounts, offchain_accounts): (Vec<_>, Vec<_>) =
            self.accounts.iter().partition(|account_header| account_header.id().is_public());

        let updated_onchain_accounts = self
            .get_updated_onchain_accounts(account_hash_updates, &onchain_accounts)
            .await?;

        let private_account_hashes = account_hash_updates
            .iter()
            .filter_map(|(account_id, _)| {
                offchain_accounts
                    .iter()
                    .find(|account| account.id() == *account_id)
                    .map(|account| (account.id(), account.hash()))
            })
            .collect::<Vec<_>>();
        Ok(AccountUpdates::new(updated_onchain_accounts, private_account_hashes))
    }

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

        let modified_notes: Vec<InputNoteRecord> = self
            .changed_notes
            .iter()
            .filter_map(|note_id| self.unspent_notes.remove(note_id))
            .collect();

        //TODO: Add output notes to update
        let note_updates = NoteUpdates::new(new_notes, vec![], modified_notes, vec![]);
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
    ) -> Result<Vec<InputNoteRecord>, ClientError> {
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

            if let Some(note_record) = self.unspent_notes.get_mut(committed_note.note_id()) {
                // The note belongs to our locally tracked set of input notes

                let inclusion_proof_received = note_record
                    .inclusion_proof_received(inclusion_proof.clone(), committed_note.metadata())?;
                let block_header_received = note_record.block_header_received(*block_header)?;

                if inclusion_proof_received || block_header_received {
                    self.changed_notes.insert(*committed_note.note_id());
                }
            } else {
                // The note is public and we are tracking it, push to the list of IDs to query
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

        let mut consumed_note_ids: BTreeMap<Nullifier, NoteId> = self
            .unspent_notes
            .values()
            .filter_map(|n| {
                nullifier_filter.contains(&n.nullifier()).then(|| (n.nullifier(), n.id()))
            })
            .collect();

        // Modify notes that were being processed by a transaciton that just got committed. These
        // notes were consumed internally.
        for transaction_update in committed_transactions {
            // Get the notes that were being processed by the transaction
            let transaction_consumed_notes: Vec<NoteId> = consumed_note_ids
                .iter()
                .filter_map(|(_, note_id)| {
                    let note_record = self.unspent_notes.get(note_id)?;
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
                // SAFETY: The note IDs in `consumed_note_ids` were extracted from the
                // `unspent_notes` map
                let input_note_record =
                    self.unspent_notes.get_mut(&note_id).expect("Note should exist");

                if input_note_record.transaction_committed(
                    transaction_update.transaction_id,
                    transaction_update.block_num,
                )? {
                    self.changed_notes.insert(note_id);

                    // Remove the note from the list so it's not modified again in the next step
                    consumed_note_ids.remove(&input_note_record.nullifier());
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
                // SAFETY: The note IDs in `consumed_note_ids` were extracted from the
                // `unspent_notes` map
                let input_note_record =
                    self.unspent_notes.get_mut(&note_id).expect("Note should exist");

                if input_note_record.is_processing() {
                    // The note was being processed by a local transaction but it was nullified
                    // externally so the transaction should be discarded
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
                        None, // TODO: Add timestamp
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
}

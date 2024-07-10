use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use core::cmp::max;

use crypto::merkle::{InOrderIndex, MmrPeaks};
use miden_objects::{
    accounts::{Account, AccountId, AccountStub},
    crypto::{self, rand::FeltRng},
    notes::{Note, NoteId, NoteInclusionProof, NoteInputs, NoteRecipient, NoteTag},
    transaction::InputNote,
    BlockHeader, Digest,
};
use miden_tx::auth::TransactionAuthenticator;
use tracing::info;
use winter_maybe_async::{maybe_async, maybe_await};

use crate::{
    rpc::{
        AccountDetails, CommittedNote, NodeRpcClient, NoteDetails, NullifierUpdate, RpcError,
        TransactionUpdate,
    },
    store::{InputNoteRecord, NoteFilter, Store, StoreError, TransactionFilter},
    Client, ClientError,
};

mod block_headers;
use block_headers::apply_mmr_changes;

mod tags;

/// Contains stats about the sync operation
pub struct SyncSummary {
    /// Block number up to which the client has been synced
    pub block_num: u32,
    /// Number of new notes received
    pub new_notes: usize,
    /// Number of tracked notes that received inclusion proofs
    pub new_inclusion_proofs: usize,
    /// Number of new nullifiers received
    pub new_nullifiers: usize,
    /// Number of on-chain accounts that have been updated
    pub updated_onchain_accounts: usize,
    /// Number of commited transactions
    pub commited_transactions: usize,
}

impl SyncSummary {
    pub fn new(
        block_num: u32,
        new_notes: usize,
        new_inclusion_proofs: usize,
        new_nullifiers: usize,
        updated_onchain_accounts: usize,
        commited_transactions: usize,
    ) -> Self {
        Self {
            block_num,
            new_notes,
            new_inclusion_proofs,
            new_nullifiers,
            updated_onchain_accounts,
            commited_transactions,
        }
    }

    pub fn new_empty(block_num: u32) -> Self {
        Self {
            block_num,
            new_notes: 0,
            new_inclusion_proofs: 0,
            new_nullifiers: 0,
            updated_onchain_accounts: 0,
            commited_transactions: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.new_notes == 0
            && self.new_inclusion_proofs == 0
            && self.new_nullifiers == 0
            && self.updated_onchain_accounts == 0
    }

    pub fn combine_with(&mut self, other: &Self) {
        self.block_num = max(self.block_num, other.block_num);
        self.new_notes += other.new_notes;
        self.new_inclusion_proofs += other.new_inclusion_proofs;
        self.new_nullifiers += other.new_nullifiers;
        self.updated_onchain_accounts += other.updated_onchain_accounts;
        self.commited_transactions += other.commited_transactions;
    }
}

impl From<&StateSyncUpdate> for SyncSummary {
    fn from(sync_update: &StateSyncUpdate) -> Self {
        let updated_output_note_ids =
            sync_update.synced_new_notes.updated_output_notes.iter().map(|(id, _)| *id);
        let updated_input_note_ids =
            sync_update.synced_new_notes.updated_input_notes.iter().map(|n| n.note().id());

        let updated_note_set: BTreeSet<NoteId> =
            updated_input_note_ids.chain(updated_output_note_ids).collect();

        SyncSummary::new(
            sync_update.block_header.block_num(),
            sync_update.synced_new_notes.new_public_notes().len(),
            updated_note_set.len(),
            sync_update.nullifiers.len(),
            sync_update.updated_onchain_accounts.len(),
            sync_update.transactions_to_commit.len(),
        )
    }
}

enum SyncStatus {
    SyncedToLastBlock(SyncSummary),
    SyncedToBlock(SyncSummary),
}

impl SyncStatus {
    pub fn sync_summary(&self) -> &SyncSummary {
        match self {
            SyncStatus::SyncedToLastBlock(summary) => summary,
            SyncStatus::SyncedToBlock(summary) => summary,
        }
    }
}

/// Contains information about new notes as consequence of a sync
pub struct SyncedNewNotes {
    /// A list of public notes that have been received on sync
    new_public_notes: Vec<InputNote>,
    /// A list of input notes corresponding to updated locally-tracked input notes
    updated_input_notes: Vec<InputNote>,
    /// A list of note IDs alongside their inclusion proofs for locally-tracked
    /// output notes
    updated_output_notes: Vec<(NoteId, NoteInclusionProof)>,
}

impl SyncedNewNotes {
    pub fn new(
        new_public_notes: Vec<InputNote>,
        updated_input_notes: Vec<InputNote>,
        updated_output_notes: Vec<(NoteId, NoteInclusionProof)>,
    ) -> Self {
        Self {
            new_public_notes,
            updated_input_notes,
            updated_output_notes,
        }
    }

    pub fn new_public_notes(&self) -> &[InputNote] {
        &self.new_public_notes
    }

    pub fn updated_input_notes(&self) -> &[InputNote] {
        &self.updated_input_notes
    }

    pub fn updated_output_notes(&self) -> &[(NoteId, NoteInclusionProof)] {
        &self.updated_output_notes
    }

    /// Returns whether no new note-related information has been retrieved
    pub fn is_empty(&self) -> bool {
        self.updated_input_notes.is_empty()
            && self.updated_output_notes.is_empty()
            && self.new_public_notes.is_empty()
    }
}

/// Contains all information needed to apply the update in the store after syncing with the node
pub struct StateSyncUpdate {
    pub block_header: BlockHeader,
    pub nullifiers: Vec<NullifierUpdate>,
    pub synced_new_notes: SyncedNewNotes,
    pub transactions_to_commit: Vec<TransactionUpdate>,
    pub new_mmr_peaks: MmrPeaks,
    pub new_authentication_nodes: Vec<(InOrderIndex, Digest)>,
    pub updated_onchain_accounts: Vec<Account>,
    pub block_has_relevant_notes: bool,
}

// CONSTANTS
// ================================================================================================

/// The number of bits to shift identifiers for in use of filters.
pub(crate) const FILTER_ID_SHIFT: u8 = 48;

impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> Client<N, R, S, A> {
    // SYNC STATE
    // --------------------------------------------------------------------------------------------

    /// Returns the block number of the last state sync block.
    #[maybe_async]
    pub fn get_sync_height(&self) -> Result<u32, ClientError> {
        maybe_await!(self.store.get_sync_height()).map_err(|err| err.into())
    }

    /// Syncs the client's state with the current state of the Miden network.
    /// Before doing so, it ensures the genesis block exists in the local store.
    ///
    /// Returns the block number the client has been synced to.
    pub async fn sync_state(&mut self) -> Result<SyncSummary, ClientError> {
        self.ensure_genesis_in_place().await?;
        let mut total_sync_summary = SyncSummary::new_empty(0);
        loop {
            let response = self.sync_state_once().await?;
            total_sync_summary.combine_with(response.sync_summary());

            if let SyncStatus::SyncedToLastBlock(_) = response {
                break;
            }
        }
        self.update_mmr_data().await?;

        Ok(total_sync_summary)
    }

    /// Updates the inclusion proof and metadata for notes that are being ignored
    /// by the client. This will not change their ignored status.
    ///
    /// This function will not update the current block number as the notes will
    /// not be updated via a sync request. Because of this, the returned
    /// [SyncSummary] will not have the corresponding block number.
    pub async fn update_ignored_notes(&mut self) -> Result<SyncSummary, ClientError> {
        let ignored_notes_ids = maybe_await!(self.get_input_notes(NoteFilter::Ignored))?
            .iter()
            .map(|note| note.id())
            .collect::<Vec<_>>();

        let note_details = self.rpc_api.get_notes_by_id(&ignored_notes_ids).await?;

        let updated_notes = note_details.len();

        let mut current_partial_mmr = maybe_await!(self.build_current_partial_mmr(true))?;
        for details in note_details {
            let note_block = self
                .get_and_store_authenticated_block(
                    details.inclusion_details().block_num,
                    &mut current_partial_mmr,
                )
                .await?;

            let note_inclusion_proof = NoteInclusionProof::new(
                note_block.block_num(),
                note_block.sub_hash(),
                note_block.note_root(),
                details.inclusion_details().note_index as u64,
                details.inclusion_details().merkle_path.clone(),
            )
            .map_err(ClientError::NoteError)?;

            maybe_await!(self
                .store
                .update_note_inclusion_proof(details.id(), note_inclusion_proof))?;
            maybe_await!(self.store.update_note_metadata(details.id(), *details.metadata()))?;
        }

        let mut sync_summary = SyncSummary::new_empty(0);
        sync_summary.new_inclusion_proofs = updated_notes;

        Ok(sync_summary)
    }

    async fn sync_state_once(&mut self) -> Result<SyncStatus, ClientError> {
        let current_block_num = maybe_await!(self.store.get_sync_height())?;

        let accounts: Vec<AccountStub> = maybe_await!(self.store.get_account_stubs())?
            .into_iter()
            .map(|(acc_stub, _)| acc_stub)
            .collect();

        let account_note_tags: Vec<NoteTag> = accounts
            .iter()
            .map(|acc| {
                NoteTag::from_account_id(acc.id(), miden_objects::notes::NoteExecutionHint::Local)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let stored_note_tags: Vec<NoteTag> = maybe_await!(self.store.get_note_tags())?;

        let expected_notes = maybe_await!(self.store.get_input_notes(NoteFilter::Expected))?;

        let uncommited_note_tags: Vec<NoteTag> = expected_notes
            .iter()
            .filter_map(|note| note.metadata().map(|metadata| metadata.tag()))
            .collect();

        let imported_tags: Vec<NoteTag> =
            expected_notes.iter().filter_map(|note| note.imported_tag()).collect();

        let note_tags: Vec<NoteTag> =
            [account_note_tags, stored_note_tags, uncommited_note_tags, imported_tags]
                .concat()
                .into_iter()
                .collect::<BTreeSet<NoteTag>>()
                .into_iter()
                .collect();

        // To receive information about added nullifiers, we reduce them to the higher 16 bits
        // Note that besides filtering by nullifier prefixes, the node also filters by block number
        // (it only returns nullifiers from current_block_num until response.block_header.block_num())
        let nullifiers_tags: Vec<u16> =
            maybe_await!(self.store.get_unspent_input_note_nullifiers())?
                .iter()
                .map(|nullifier| (nullifier.inner()[3].as_int() >> FILTER_ID_SHIFT) as u16)
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

        let new_note_details =
            self.get_note_details(response.note_inclusions, &response.block_header).await?;

        let incoming_block_has_relevant_notes =
            self.check_block_relevance(&new_note_details).await?;

        let (onchain_accounts, offchain_accounts): (Vec<_>, Vec<_>) =
            accounts.into_iter().partition(|account_stub| account_stub.id().is_on_chain());

        let updated_onchain_accounts = self
            .get_updated_onchain_accounts(&response.account_hash_updates, &onchain_accounts)
            .await?;

        self.validate_local_account_hashes(&response.account_hash_updates, &offchain_accounts)
            .await?;

        // Derive new nullifiers data
        let new_nullifiers = maybe_await!(self.get_new_nullifiers(response.nullifiers))?;

        // Build PartialMmr with current data and apply updates
        let (new_peaks, new_authentication_nodes) = {
            let current_partial_mmr = maybe_await!(self.build_current_partial_mmr(false))?;

            let (current_block, has_relevant_notes) =
                maybe_await!(self.store.get_block_header_by_num(current_block_num))?;

            apply_mmr_changes(
                current_partial_mmr,
                response.mmr_delta,
                current_block,
                has_relevant_notes,
            )?
        };

        let transactions_to_commit =
            maybe_await!(self.get_transactions_to_commit(response.transactions))?;

        let state_sync_update = StateSyncUpdate {
            block_header: response.block_header,
            nullifiers: new_nullifiers,
            synced_new_notes: new_note_details,
            transactions_to_commit,
            new_mmr_peaks: new_peaks,
            new_authentication_nodes,
            updated_onchain_accounts: updated_onchain_accounts.clone(),
            block_has_relevant_notes: incoming_block_has_relevant_notes,
        };

        // Store summary to return later
        let sync_summary = SyncSummary::from(&state_sync_update);

        // Apply received and computed updates to the store
        maybe_await!(self.store.apply_state_sync(state_sync_update))
            .map_err(ClientError::StoreError)?;

        if response.chain_tip == response.block_header.block_num() {
            Ok(SyncStatus::SyncedToLastBlock(sync_summary))
        } else {
            Ok(SyncStatus::SyncedToBlock(sync_summary))
        }
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Extracts information about notes that the client is interested in, creating the note inclusion
    /// proof in order to correctly update store data
    async fn get_note_details(
        &mut self,
        committed_notes: Vec<CommittedNote>,
        block_header: &BlockHeader,
    ) -> Result<SyncedNewNotes, ClientError> {
        // We'll only pick committed notes that we are tracking as input/output notes. Since the
        // sync response contains notes matching either the provided accounts or the provided tag
        // we might get many notes when we only care about a few of those.

        let mut new_public_notes = vec![];
        let mut tracked_input_notes = vec![];
        let mut tracked_output_notes_proofs = vec![];

        let mut expected_input_notes: BTreeMap<NoteId, InputNoteRecord> =
            maybe_await!(self.store.get_input_notes(NoteFilter::Expected))?
                .into_iter()
                .map(|n| (n.id(), n))
                .collect();

        expected_input_notes.extend(
            maybe_await!(self.store.get_input_notes(NoteFilter::Processing))?
                .into_iter()
                .map(|input_note| (input_note.id(), input_note)),
        );

        let mut expected_output_notes: BTreeSet<NoteId> =
            maybe_await!(self.store.get_output_notes(NoteFilter::Expected))?
                .into_iter()
                .map(|n| n.id())
                .collect();

        expected_output_notes.extend(
            maybe_await!(self.store.get_output_notes(NoteFilter::Processing))?
                .into_iter()
                .map(|output_note| output_note.id()),
        );

        for committed_note in committed_notes {
            if let Some(note_record) = expected_input_notes.get(committed_note.note_id()) {
                // The note belongs to our locally tracked set of expected notes, build the inclusion proof
                let note_inclusion_proof = NoteInclusionProof::new(
                    block_header.block_num(),
                    block_header.sub_hash(),
                    block_header.note_root(),
                    committed_note.note_index().into(),
                    committed_note.merkle_path().clone(),
                )?;

                let note_inputs = NoteInputs::new(note_record.details().inputs().clone())?;
                let note_recipient = NoteRecipient::new(
                    note_record.details().serial_num(),
                    note_record.details().script().clone(),
                    note_inputs,
                );
                let note = Note::new(
                    note_record.assets().clone(),
                    committed_note.metadata(),
                    note_recipient,
                );

                let input_note = InputNote::authenticated(note, note_inclusion_proof);

                tracked_input_notes.push(input_note);
            }

            if expected_output_notes.contains(committed_note.note_id()) {
                let note_id_with_inclusion_proof = NoteInclusionProof::new(
                    block_header.block_num(),
                    block_header.sub_hash(),
                    block_header.note_root(),
                    committed_note.note_index().into(),
                    committed_note.merkle_path().clone(),
                )
                .map(|note_inclusion_proof| (*committed_note.note_id(), note_inclusion_proof))?;

                tracked_output_notes_proofs.push(note_id_with_inclusion_proof);
            }

            if !expected_input_notes.contains_key(committed_note.note_id())
                && !expected_output_notes.contains(committed_note.note_id())
            {
                // The note is public and we are not tracking it, push to the list of IDs to query
                new_public_notes.push(*committed_note.note_id());
            }
        }

        // Query the node for input note data and build the entities
        let new_public_notes =
            self.fetch_public_note_details(&new_public_notes, block_header).await?;

        Ok(SyncedNewNotes::new(
            new_public_notes,
            tracked_input_notes,
            tracked_output_notes_proofs,
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
    ) -> Result<Vec<InputNote>, ClientError> {
        if query_notes.is_empty() {
            return Ok(vec![]);
        }
        info!("Getting note details for notes that are not being tracked.");

        let notes_data = self.rpc_api.get_notes_by_id(query_notes).await?;
        let mut return_notes = Vec::with_capacity(query_notes.len());
        for note_data in notes_data {
            match note_data {
                NoteDetails::OffChain(id, ..) => {
                    // TODO: Is there any benefit to not ignoring these? In any case we do not have
                    // the recipient which is mandatory right now.
                    info!("Note {} is private but the client is not tracking it, ignoring.", id);
                },
                NoteDetails::Public(note, inclusion_proof) => {
                    info!("Retrieved details for Note ID {}.", note.id());
                    let note_inclusion_proof = NoteInclusionProof::new(
                        block_header.block_num(),
                        block_header.sub_hash(),
                        block_header.note_root(),
                        inclusion_proof.note_index as u64,
                        inclusion_proof.merkle_path,
                    )
                    .map_err(ClientError::NoteError)?;

                    return_notes.push(InputNote::authenticated(note, note_inclusion_proof))
                },
            }
        }
        Ok(return_notes)
    }

    /// Extracts information about nullifiers for unspent input notes that the client is tracking
    /// from the received list of nullifiers in the sync response
    #[maybe_async]
    fn get_new_nullifiers(
        &self,
        mut new_nullifiers: Vec<NullifierUpdate>,
    ) -> Result<Vec<NullifierUpdate>, ClientError> {
        // Get current unspent nullifiers
        let nullifiers = maybe_await!(self.store.get_unspent_input_note_nullifiers())?;

        new_nullifiers.retain(|nullifier_update| nullifiers.contains(&nullifier_update.nullifier));

        Ok(new_nullifiers)
    }

    /// Extracts information about transactions for uncommitted transactions that the client is tracking
    /// from the received [SyncStateResponse]
    #[maybe_async]
    fn get_transactions_to_commit(
        &self,
        mut transactions: Vec<TransactionUpdate>,
    ) -> Result<Vec<TransactionUpdate>, ClientError> {
        // Get current uncommitted transactions
        let uncommitted_transaction_ids =
            maybe_await!(self.store.get_transactions(TransactionFilter::Uncomitted))?
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
        current_onchain_accounts: &[AccountStub],
    ) -> Result<Vec<Account>, ClientError> {
        let mut accounts_to_update: Vec<Account> = Vec::new();
        for (remote_account_id, remote_account_hash) in account_updates {
            // check if this updated account is tracked by the client
            let current_account = current_onchain_accounts
                .iter()
                .find(|acc| *remote_account_id == acc.id() && *remote_account_hash != acc.hash());

            if let Some(tracked_account) = current_account {
                info!("On-chain account hash difference detected for account with ID: {}. Fetching node for updates...", tracked_account.id());
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

    /// Validates account hash updates and returns an error if there is a mismatch.
    async fn validate_local_account_hashes(
        &mut self,
        account_updates: &[(AccountId, Digest)],
        current_offchain_accounts: &[AccountStub],
    ) -> Result<(), ClientError> {
        for (remote_account_id, remote_account_hash) in account_updates {
            // ensure that if we track that account, it has the same hash
            let mismatched_accounts = current_offchain_accounts
                .iter()
                .find(|acc| *remote_account_id == acc.id() && *remote_account_hash != acc.hash());

            // OffChain accounts should always have the latest known state. If we receive a stale
            // update we ignore it.
            if mismatched_accounts.is_some() {
                let current_account =
                    maybe_await!(self.store.get_account_stub(*remote_account_id))?.0;
                let account_by_hash =
                    maybe_await!(self.store.get_account_stub_by_hash(*remote_account_hash))?;

                if account_by_hash.is_none()
                    || account_by_hash.expect("account should be some").nonce()
                        == current_account.nonce()
                {
                    return Err(StoreError::AccountHashMismatch(*remote_account_id).into());
                }
            }
        }
        Ok(())
    }
}

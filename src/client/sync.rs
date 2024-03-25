use crypto::merkle::{InOrderIndex, MmrDelta, MmrPeaks, PartialMmr};
use miden_objects::{
    accounts::{AccountId, AccountStub},
    crypto,
    notes::{NoteId, NoteInclusionProof},
    transaction::TransactionId,
    BlockHeader, Digest,
};
use tracing::warn;

use super::{
    rpc::{CommittedNote, NodeRpcClient},
    transactions::TransactionRecord,
    Client,
};
use crate::{
    errors::{ClientError, StoreError},
    store::{ChainMmrNodeFilter, NoteFilter, Store, TransactionFilter},
};

pub enum SyncStatus {
    SyncedToLastBlock(u32),
    SyncedToBlock(u32),
}

// CONSTANTS
// ================================================================================================

/// The number of bits to shift identifiers for in use of filters.
pub const FILTER_ID_SHIFT: u8 = 48;

impl<N: NodeRpcClient, S: Store> Client<N, S> {
    // SYNC STATE
    // --------------------------------------------------------------------------------------------

    /// Returns the block number of the last state sync block.
    pub fn get_sync_height(&self) -> Result<u32, ClientError> {
        self.store.get_sync_height().map_err(|err| err.into())
    }

    /// Returns the list of note tags tracked by the client.
    pub fn get_note_tags(&self) -> Result<Vec<u64>, ClientError> {
        self.store.get_note_tags().map_err(|err| err.into())
    }

    /// Adds a note tag for the client to track.
    pub fn add_note_tag(
        &mut self,
        tag: u64,
    ) -> Result<(), ClientError> {
        match self.store.add_note_tag(tag).map_err(|err| err.into()) {
            Ok(true) => Ok(()),
            Ok(false) => {
                warn!("Tag {} is already being tracked", tag);
                Ok(())
            },
            Err(err) => Err(err),
        }
    }

    /// Syncs the client's state with the current state of the Miden network.
    /// Before doing so, it ensures the genesis block exists in the local store.
    ///
    /// Returns the block number the client has been synced to.
    pub async fn sync_state(&mut self) -> Result<u32, ClientError> {
        self.ensure_genesis_in_place().await?;
        loop {
            let response = self.sync_state_once().await?;
            if let SyncStatus::SyncedToLastBlock(v) = response {
                return Ok(v);
            }
        }
    }

    /// Attempts to retrieve the genesis block from the store. If not found,
    /// it requests it from the node and store it.
    async fn ensure_genesis_in_place(&mut self) -> Result<(), ClientError> {
        let genesis = self.store.get_block_header_by_num(0);

        match genesis {
            Ok(_) => Ok(()),
            Err(StoreError::BlockHeaderNotFound(0)) => self.retrieve_and_store_genesis().await,
            Err(err) => Err(ClientError::StoreError(err)),
        }
    }

    /// Calls `get_block_header_by_number` requesting the genesis block and storing it
    /// in the local database
    async fn retrieve_and_store_genesis(&mut self) -> Result<(), ClientError> {
        let genesis_block = self.rpc_api.get_block_header_by_number(Some(0)).await?;

        let blank_mmr_peaks =
            MmrPeaks::new(0, vec![]).expect("Blank MmrPeaks should not fail to instantiate");
        // NOTE: If genesis block data ever includes notes in the future, the third parameter in
        // this `insert_block_header` call may be `true`
        self.store.insert_block_header(genesis_block, blank_mmr_peaks, false)?;
        Ok(())
    }

    async fn sync_state_once(&mut self) -> Result<SyncStatus, ClientError> {
        let current_block_num = self.store.get_sync_height()?;

        let accounts: Vec<AccountStub> = self
            .store
            .get_account_stubs()?
            .into_iter()
            .map(|(acc_stub, _)| acc_stub)
            .collect();

        let note_tags: Vec<u16> = accounts
            .iter()
            .map(|acc| ((u64::from(acc.id()) >> FILTER_ID_SHIFT) as u16))
            .collect();

        // To receive information about added nullifiers, we reduce them to the higher 16 bits
        // Note that besides filtering by nullifier prefixes, the node also filters by block number
        // (it only returns nullifiers from current_block_num until response.block_header.block_num())
        let nullifiers_tags: Vec<u16> = self
            .store
            .get_unspent_input_note_nullifiers()?
            .iter()
            .map(|nullifier| (nullifier.inner()[3].as_int() >> FILTER_ID_SHIFT) as u16)
            .collect();

        // Send request
        let account_ids: Vec<AccountId> = accounts.iter().map(|acc| acc.id()).collect();
        let response = self
            .rpc_api
            .sync_state(current_block_num, &account_ids, &note_tags, &nullifiers_tags)
            .await?;

        // We don't need to continue if the chain has not advanced
        if response.block_header.block_num() == current_block_num {
            return Ok(SyncStatus::SyncedToLastBlock(current_block_num));
        }

        let committed_notes =
            self.build_inclusion_proofs(response.note_inclusions, &response.block_header)?;

        // Check if the returned account hashes match latest account hashes in the database
        check_account_hashes(&response.account_hash_updates, &accounts)?;

        // Derive new nullifiers data
        let new_nullifiers = self.get_new_nullifiers(response.nullifiers)?;

        // Build PartialMmr with current data and apply updates
        let (new_peaks, new_authentication_nodes) = {
            let current_partial_mmr = self.build_current_partial_mmr()?;

            let (current_block, has_relevant_notes) =
                self.store.get_block_header_by_num(current_block_num)?;

            apply_mmr_changes(
                current_partial_mmr,
                response.mmr_delta,
                current_block,
                has_relevant_notes,
            )?
        };

        let note_ids: Vec<NoteId> = committed_notes.iter().map(|(id, _)| (*id)).collect();

        let uncommitted_transactions =
            self.store.get_transactions(TransactionFilter::Uncomitted)?;

        let transactions_to_commit = get_transactions_to_commit(
            &uncommitted_transactions,
            &note_ids,
            &new_nullifiers,
            &response.account_hash_updates,
        );

        // Apply received and computed updates to the store
        self.store
            .apply_state_sync(
                response.block_header,
                new_nullifiers,
                committed_notes,
                &transactions_to_commit,
                new_peaks,
                &new_authentication_nodes,
            )
            .map_err(ClientError::StoreError)?;

        if response.chain_tip == response.block_header.block_num() {
            Ok(SyncStatus::SyncedToLastBlock(response.chain_tip))
        } else {
            Ok(SyncStatus::SyncedToBlock(response.block_header.block_num()))
        }
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Extracts information about notes that the client is interested in, creating the note inclusion
    /// proof in order to correctly update store data
    fn build_inclusion_proofs(
        &self,
        committed_notes: Vec<CommittedNote>,
        block_header: &BlockHeader,
    ) -> Result<Vec<(NoteId, NoteInclusionProof)>, ClientError> {
        // We'll only pick committed notes that we are tracking as input/output notes. Since the
        // sync response contains notes matching either the provided accounts or the provided tag
        // we might get many notes when we only care about a few of those.
        let pending_input_notes: Vec<NoteId> = self
            .store
            .get_input_notes(NoteFilter::Pending)?
            .iter()
            .map(|n| n.id())
            .collect();

        let pending_output_notes: Vec<NoteId> = self
            .store
            .get_output_notes(NoteFilter::Pending)?
            .iter()
            .map(|n| n.id())
            .collect();

        let mut pending_notes = [pending_input_notes, pending_output_notes].concat();
        pending_notes.dedup();

        committed_notes
            .iter()
            .filter_map(|commited_note| {
                if pending_notes.contains(commited_note.note_id()) {
                    // FIXME: This removal is to accomodate a problem with how the node constructs paths where
                    // they are constructed using note ID instead of authentication hash, so for now we remove the first
                    // node here.
                    //
                    // See: https://github.com/0xPolygonMiden/miden-node/blob/main/store/src/state.rs#L274
                    let mut merkle_path = commited_note.merkle_path().clone();
                    if merkle_path.len() > 0 {
                        let _ = merkle_path.remove(0);
                    }

                    let note_inclusion_proof = NoteInclusionProof::new(
                        block_header.block_num(),
                        block_header.sub_hash(),
                        block_header.note_root(),
                        commited_note.note_index().into(),
                        merkle_path,
                    )
                    .map_err(ClientError::NoteError)
                    .map(|proof| (*commited_note.note_id(), proof));

                    Some(note_inclusion_proof)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Builds the current view of the chain's [PartialMmr]. Because we want to add all new
    /// authentication nodes that could come from applying the MMR updates, we need to track all
    /// known leaves thus far.
    ///
    /// As part of the syncing process, we add the current block number so we don't need to
    /// track it here.
    pub(crate) fn build_current_partial_mmr(&self) -> Result<PartialMmr, ClientError> {
        let current_block_num = self.store.get_sync_height()?;

        let tracked_nodes = self.store.get_chain_mmr_nodes(ChainMmrNodeFilter::All)?;
        let current_peaks = self.store.get_chain_mmr_peaks_by_block_num(current_block_num)?;

        let track_latest = if current_block_num != 0 {
            match self.store.get_block_header_by_num(current_block_num - 1) {
                Ok((_, previous_block_had_notes)) => Ok(previous_block_had_notes),
                Err(StoreError::BlockHeaderNotFound(_)) => Ok(false),
                Err(err) => Err(ClientError::StoreError(err)),
            }?
        } else {
            false
        };

        Ok(PartialMmr::from_parts(current_peaks, tracked_nodes, track_latest))
    }

    /// Extracts information about nullifiers for unspent input notes that the client is tracking
    /// from the received [SyncStateResponse]
    fn get_new_nullifiers(
        &self,
        new_nullifiers: Vec<Digest>,
    ) -> Result<Vec<Digest>, ClientError> {
        // Get current unspent nullifiers
        let nullifiers = self
            .store
            .get_unspent_input_note_nullifiers()?
            .iter()
            .map(|nullifier| nullifier.inner())
            .collect::<Vec<_>>();

        let new_nullifiers = new_nullifiers
            .into_iter()
            .filter(|nullifier| nullifiers.contains(nullifier))
            .collect();

        Ok(new_nullifiers)
    }
}

// UTILS
// --------------------------------------------------------------------------------------------

/// Applies changes to the Mmr structure, storing authentication nodes for leaves we track
/// and returns the updated [PartialMmr]
fn apply_mmr_changes(
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

/// Validates account hash updates and returns an error if there is a mismatch.
fn check_account_hashes(
    account_updates: &[(AccountId, Digest)],
    current_accounts: &[AccountStub],
) -> Result<(), StoreError> {
    for (remote_account_id, remote_account_hash) in account_updates {
        {
            if let Some(local_account) =
                current_accounts.iter().find(|acc| *remote_account_id == acc.id())
            {
                if *remote_account_hash != local_account.hash() {
                    return Err(StoreError::AccountHashMismatch(*remote_account_id));
                }
            }
        }
    }
    Ok(())
}

/// Returns the list of transactions that should be marked as committed based on the state update info
///
/// To set an uncommitted transaction as committed three things must hold:
///
/// - All of the transaction's output notes are committed
/// - All of the transaction's input notes are consumed, which means we got their nullifiers as
/// part of the update
/// - The account corresponding to the transaction hash matches the transaction's
// final_account_state
fn get_transactions_to_commit(
    uncommitted_transactions: &[TransactionRecord],
    note_ids: &[NoteId],
    nullifiers: &[Digest],
    account_hash_updates: &[(AccountId, Digest)],
) -> Vec<TransactionId> {
    uncommitted_transactions
        .iter()
        .filter(|t| {
            // TODO: based on the discussion in
            // https://github.com/0xPolygonMiden/miden-client/issues/144, we should be aware
            // that in the future it'll be possible to have many transactions modifying an
            // account be included in a single block. If that happens, we'll need to rewrite
            // this check
            t.input_note_nullifiers.iter().all(|n| nullifiers.contains(n))
                && t.output_notes.iter().all(|n| note_ids.contains(&n.id()))
                && account_hash_updates.iter().any(|(account_id, account_hash)| {
                    *account_id == t.account_id && *account_hash == t.final_account_state
                })
        })
        .map(|t| t.id)
        .collect()
}

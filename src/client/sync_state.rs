use super::Client;
use crypto::merkle::{InOrderIndex, MerklePath, MmrDelta, MmrPeaks, PartialMmr};
use miden_node_proto::{
    account::AccountId as ProtoAccountId,
    note::NoteSyncRecord,
    requests::{GetBlockHeaderByNumberRequest, SyncStateRequest},
    responses::{AccountHashUpdate, SyncStateResponse},
};

use objects::{
    accounts::AccountStub, crypto, notes::NoteInclusionProof, BlockHeader, Digest, StarkField,
};

use crate::{
    errors::{ClientError, StoreError},
    store::{chain_data::ChainMmrNodeFilter, Store},
};

pub enum SyncStatus {
    SyncedToLastBlock(u32),
    SyncedToBlock(u32),
}

// CONSTANTS
// ================================================================================================

/// The number of bits to shift identifiers for in use of filters.
pub const FILTER_ID_SHIFT: u8 = 48;

impl Client {
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
    pub fn add_note_tag(&mut self, tag: u64) -> Result<(), ClientError> {
        match self.store.add_note_tag(tag).map_err(|err| err.into()) {
            Ok(true) => Ok(()),
            Ok(false) => {
                println!("tag {} is already being tracked", tag);
                Ok(())
            }
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
            let response = self.single_sync_state().await?;
            if let SyncStatus::SyncedToLastBlock(v) = response {
                return Ok(v);
            }
        }
    }

    /// Attempts to retrieve the genesis block from the store. If not found,
    /// it requests it from the node and store it
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
        let genesis_block = self
            .rpc_api
            .get_block_header_by_number(GetBlockHeaderByNumberRequest { block_num: Some(0) })
            .await
            .map_err(ClientError::RpcApiError)?
            .into_inner();

        let genesis_block: objects::BlockHeader = genesis_block
            .block_header
            .ok_or(ClientError::RpcExpectedFieldMissing(
                "Expected block header in genesis block request".to_string(),
            ))?
            .try_into()?;

        let tx = self.store.db.transaction()?;

        Store::insert_block_header(
            &tx,
            genesis_block,
            MmrPeaks::new(0, vec![]).expect("Blank MmrPeaks"),
            false,
        )?;

        tx.commit()?;
        Ok(())
    }

    async fn single_sync_state(&mut self) -> Result<SyncStatus, ClientError> {
        // Construct request
        let current_block_num = self.store.get_sync_height()?;

        let accounts: Vec<AccountStub> = self
            .store
            .get_accounts()?
            .into_iter()
            .map(|(acc, _)| acc)
            .collect();

        let note_tags: Vec<u32> = accounts
            .iter()
            .map(|acc| (u64::from(acc.id()) >> FILTER_ID_SHIFT) as u32)
            .collect::<Vec<_>>();

        let nullifiers = self.store.get_unspent_input_note_nullifiers()?;

        // Send request and convert types
        let response = self
            .sync_state_request(current_block_num, &accounts, note_tags, &nullifiers)
            .await?;

        let incoming_block_header = response
            .block_header
            .as_ref()
            .ok_or(ClientError::RpcExpectedFieldMissing("BlockHeader".into()))?;

        // We don't need to continue if the chain has not advanced
        if incoming_block_header.block_num == current_block_num {
            return Ok(SyncStatus::SyncedToLastBlock(current_block_num));
        }

        let incoming_block_header: BlockHeader = incoming_block_header.try_into()?;

        let committed_notes =
            self.get_newly_committed_note_info(&response.notes, &incoming_block_header)?;

        // Check if the returned account hashes match latest account hashes in the database
        check_account_hashes(&response.accounts, &accounts)?;

        // Derive new nullifiers data
        let new_nullifiers = self.get_new_nullifiers(&response)?;

        let mmr_delta: crypto::merkle::MmrDelta = response
            .mmr_delta
            .ok_or(ClientError::RpcExpectedFieldMissing("MmrDelta".into()))?
            .try_into()?;

        // Build PartialMmr with current data and apply updates
        let (new_peaks, new_authentication_nodes) = {
            let current_partial_mmr = self.build_partial_mmr_for_block(current_block_num)?;

            let (current_block, has_relevant_notes) =
                self.store.get_block_header_by_num(current_block_num)?;

            apply_mmr_changes(
                current_partial_mmr,
                mmr_delta,
                current_block,
                has_relevant_notes,
            )?
        };

        // Apply received and computed updates to the store
        self.store
            .apply_state_sync(
                incoming_block_header,
                new_nullifiers,
                committed_notes,
                new_peaks,
                &new_authentication_nodes,
            )
            .map_err(ClientError::StoreError)?;

        if response.chain_tip == incoming_block_header.block_num() {
            Ok(SyncStatus::SyncedToLastBlock(response.chain_tip))
        } else {
            Ok(SyncStatus::SyncedToBlock(incoming_block_header.block_num()))
        }
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Extracts information about notes that the client is interested in, creating the note inclusion
    /// proof in order to correctly update store data
    fn get_newly_committed_note_info(
        &self,
        notes: &[NoteSyncRecord],
        block_header: &BlockHeader,
    ) -> Result<Vec<(Digest, NoteInclusionProof)>, ClientError> {
        let pending_notes: Vec<Digest> = self
            .store
            .get_input_notes(crate::store::notes::InputNoteFilter::Pending)?
            .iter()
            .map(|n| n.note().id().inner())
            .collect();

        let notes_with_hashes_and_merkle_paths = notes
            .iter()
            .map(|note_record| {
                // Handle Options first
                let note_hash = note_record
                    .note_hash
                    .clone()
                    .ok_or(ClientError::RpcExpectedFieldMissing("NoteHash".into()))?;
                let note_merkle_path = note_record
                    .merkle_path
                    .clone()
                    .ok_or(ClientError::RpcExpectedFieldMissing("MerklePath".into()))?;
                // Handle casting after
                let note_hash = note_hash.try_into()?;
                let merkle_path: crypto::merkle::MerklePath = note_merkle_path.try_into()?;

                Ok((note_record, note_hash, merkle_path))
            })
            .collect::<Result<Vec<_>, ClientError>>()?;

        notes_with_hashes_and_merkle_paths
            .iter()
            .filter_map(|(note, note_id, merkle_path)| {
                if pending_notes.contains(note_id) {
                    // FIXME: This removal is to accomodate a problem with how the node constructs paths where
                    // they are constructed using note ID instead of authentication hash, so for now we remove the first
                    // node here.
                    //
                    // See: https://github.com/0xPolygonMiden/miden-node/blob/main/store/src/state.rs#L274
                    let mut merkle_path = merkle_path.clone();
                    if merkle_path.len() > 0 {
                        let _ = merkle_path.remove(0);
                    }
                    let note_id_and_proof = NoteInclusionProof::new(
                        block_header.block_num(),
                        block_header.sub_hash(),
                        block_header.note_root(),
                        note.note_index.into(),
                        merkle_path,
                    )
                    .map_err(ClientError::NoteError)
                    .map(|proof| (*note_id, proof));

                    Some(note_id_and_proof)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Sends a sync state request to the Miden node and returns the response.
    async fn sync_state_request(
        &mut self,
        block_num: u32,
        account_ids: &[AccountStub],
        note_tags: Vec<u32>,
        nullifiers: &[Digest],
    ) -> Result<SyncStateResponse, ClientError> {
        let account_ids = account_ids
            .iter()
            .map(|acc| ProtoAccountId {
                id: u64::from(acc.id()),
            })
            .collect();

        let nullifiers = nullifiers
            .iter()
            .map(|nullifier| (nullifier[3].as_int() >> FILTER_ID_SHIFT) as u32)
            .collect();

        let request = SyncStateRequest {
            block_num,
            account_ids,
            note_tags,
            nullifiers,
        };

        Ok(self.rpc_api.sync_state(request).await?.into_inner())
    }

    /// Builds the current view of the chain's [PartialMmr]. Because we want to add all new
    /// authentication nodes that could come from applying the MMR updates, we need to track all
    /// known leaves thus far.
    ///
    /// As part of the syncing process, we add the current block number so we don't need to
    /// add it here.
    fn build_partial_mmr_for_block(&self, block_num: u32) -> Result<PartialMmr, ClientError> {
        let tracked_nodes = self.store.get_chain_mmr_nodes(ChainMmrNodeFilter::All)?;
        let current_peaks = self.store.get_chain_mmr_peaks_by_block_num(block_num)?;
        let tracked_blocks = self.store.get_tracked_block_headers()?;
        let mut partial_mmr = PartialMmr::from_peaks(current_peaks);

        for block in tracked_blocks {
            if block.block_num() as usize >= partial_mmr.forest() {
                continue;
            }

            let mut merkle_nodes = Vec::new();
            let mut idx = InOrderIndex::from_leaf_pos(block.block_num() as usize);

            while let Some(node) = tracked_nodes.get(&idx.sibling()) {
                merkle_nodes.push(*node);
                idx = idx.parent();
            }

            let merkle_path = MerklePath::new(merkle_nodes);
            // Track the relevant block with the constructed merkle paths
            partial_mmr
                .track(block.block_num() as usize, block.hash(), &merkle_path)
                .map_err(StoreError::MmrError)?;
        }

        Ok(partial_mmr)
    }

    /// Extracts information about nullifiers for unspent input notes that the client is tracking
    /// from the received [SyncStateResponse]
    fn get_new_nullifiers(
        &self,
        sync_state_response: &SyncStateResponse,
    ) -> Result<Vec<Digest>, ClientError> {
        // Get current unspent nullifiers
        let nullifiers = self.store.get_unspent_input_note_nullifiers()?;

        // Get NullifierUpdates
        let response_nullifiers = sync_state_response
            .nullifiers
            .clone()
            .into_iter()
            .map(|x| {
                x.nullifier
                    .ok_or(ClientError::RpcExpectedFieldMissing("Nullifier".into()))
            })
            .collect::<Result<Vec<_>, ClientError>>()?;

        let parsed_new_nullifiers = response_nullifiers
            .into_iter()
            .map(|response_nullifier| {
                response_nullifier
                    .try_into()
                    .map_err(ClientError::RpcTypeConversionFailure)
            })
            .collect::<Result<Vec<_>, ClientError>>()?;

        let new_nullifiers = parsed_new_nullifiers
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
    // TODO: reload local full view of Partial Mmr here
    let mut partial_mmr: PartialMmr = current_partial_mmr;

    // First, apply curent_block to the Mmr
    let new_authentication_nodes = partial_mmr
        .add(
            current_block_header.hash(),
            current_block_has_relevant_notes,
        )
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
    account_updates: &[AccountHashUpdate],
    current_accounts: &[AccountStub],
) -> Result<(), StoreError> {
    for account_update in account_updates {
        if let (Some(update_account_id), Some(remote_account_hash)) =
            (&account_update.account_id, &account_update.account_hash)
        {
            let update_account_id: u64 = update_account_id.clone().into();
            if let Some(acc_stub) = current_accounts
                .iter()
                .find(|acc| update_account_id == u64::from(acc.id()))
            {
                let remote_account_hash: Digest = remote_account_hash
                    .try_into()
                    .map_err(StoreError::RpcTypeConversionFailure)?;

                if remote_account_hash != acc_stub.hash() {
                    return Err(StoreError::AccountHashMismatch(
                        update_account_id
                            .try_into()
                            .map_err(StoreError::AccountError)?,
                    ));
                }
            }
        }
    }
    Ok(())
}

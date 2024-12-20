use alloc::vec::Vec;

use crypto::merkle::{InOrderIndex, MmrDelta, MmrPeaks, PartialMmr};
use miden_objects::{
    crypto::{self, merkle::MerklePath, rand::FeltRng},
    BlockHeader, Digest,
};
use tracing::warn;

use super::NoteUpdates;
use crate::{
    notes::NoteScreener,
    store::{ChainMmrNodeFilter, NoteFilter, StoreError},
    Client, ClientError,
};

/// Network information management methods.
impl<R: FeltRng> Client<R> {
    /// Updates committed notes with no MMR data. These could be notes that were
    /// imported with an inclusion proof, but its block header isn't tracked.
    pub(crate) async fn update_mmr_data(&mut self) -> Result<(), ClientError> {
        let mut current_partial_mmr = self.build_current_partial_mmr(true).await?;

        let mut changed_notes = vec![];
        for mut note in self.store.get_input_notes(NoteFilter::Unverified).await? {
            let block_num = note
                .inclusion_proof()
                .expect("Commited notes should have inclusion proofs")
                .location()
                .block_num();
            let block_header = self
                .get_and_store_authenticated_block(block_num, &mut current_partial_mmr)
                .await?;

            if note.block_header_received(block_header)? {
                changed_notes.push(note);
            }
        }

        self.store.upsert_input_notes(&changed_notes).await?;

        Ok(())
    }

    /// Attempts to retrieve the genesis block from the store. If not found,
    /// it requests it from the node and store it.
    pub(crate) async fn ensure_genesis_in_place(&mut self) -> Result<(), ClientError> {
        let genesis = self.store.get_block_header_by_num(0).await;

        match genesis {
            Ok(_) => Ok(()),
            Err(StoreError::BlockHeaderNotFound(0)) => self.retrieve_and_store_genesis().await,
            Err(err) => Err(ClientError::StoreError(err)),
        }
    }

    /// Calls `get_block_header_by_number` requesting the genesis block and storing it
    /// in the local database.
    async fn retrieve_and_store_genesis(&mut self) -> Result<(), ClientError> {
        let (genesis_block, _) = self.rpc_api.get_block_header_by_number(Some(0), false).await?;

        let blank_mmr_peaks =
            MmrPeaks::new(0, vec![]).expect("Blank MmrPeaks should not fail to instantiate");
        // NOTE: If genesis block data ever includes notes in the future, the third parameter in
        // this `insert_block_header` call may be `true`
        self.store.insert_block_header(genesis_block, blank_mmr_peaks, false).await?;
        Ok(())
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

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

    /// Builds the current view of the chain's [PartialMmr]. Because we want to add all new
    /// authentication nodes that could come from applying the MMR updates, we need to track all
    /// known leaves thus far.
    ///
    /// As part of the syncing process, we add the current block number so we don't need to
    /// track it here.
    pub(crate) async fn build_current_partial_mmr(
        &self,
        include_current_block: bool,
    ) -> Result<PartialMmr, ClientError> {
        let current_block_num = self.store.get_sync_height().await?;

        let tracked_nodes = self.store.get_chain_mmr_nodes(ChainMmrNodeFilter::All).await?;
        let current_peaks = self.store.get_chain_mmr_peaks_by_block_num(current_block_num).await?;

        let track_latest = if current_block_num != 0 {
            match self.store.get_block_header_by_num(current_block_num - 1).await {
                Ok((_, previous_block_had_notes)) => Ok(previous_block_had_notes),
                Err(StoreError::BlockHeaderNotFound(_)) => Ok(false),
                Err(err) => Err(ClientError::StoreError(err)),
            }?
        } else {
            false
        };

        let mut current_partial_mmr =
            PartialMmr::from_parts(current_peaks, tracked_nodes, track_latest);

        if include_current_block {
            let (current_block, has_client_notes) =
                self.store.get_block_header_by_num(current_block_num).await?;

            current_partial_mmr.add(current_block.hash(), has_client_notes);
        }

        Ok(current_partial_mmr)
    }

    /// Retrieves and stores a [BlockHeader] by number, and stores its authentication data as well.
    ///
    /// If the store already contains MMR data for the requested block number, the request isn't
    /// done and the stored block header is returned.
    pub(crate) async fn get_and_store_authenticated_block(
        &mut self,
        block_num: u32,
        current_partial_mmr: &mut PartialMmr,
    ) -> Result<BlockHeader, ClientError> {
        if current_partial_mmr.is_tracked(block_num as usize) {
            warn!("Current partial MMR already contains the requested data");
            let (block_header, _) = self.store.get_block_header_by_num(block_num).await?;
            return Ok(block_header);
        }
        let (block_header, mmr_proof) =
            self.rpc_api.get_block_header_by_number(Some(block_num), true).await?;

        let mmr_proof = mmr_proof
            .expect("NodeRpcApi::get_block_header_by_number() should have returned an MMR proof");
        // Trim merkle path to keep nodes relevant to our current PartialMmr since the node's MMR
        // might be of a forest arbitrarily higher
        let path_nodes = adjust_merkle_path_for_forest(
            &mmr_proof.merkle_path,
            block_num as usize,
            current_partial_mmr.forest(),
        );

        let merkle_path = MerklePath::new(path_nodes.iter().map(|(_, n)| *n).collect());

        current_partial_mmr
            .track(block_num as usize, block_header.hash(), &merkle_path)
            .map_err(StoreError::MmrError)?;

        // Insert header and MMR nodes
        self.store
            .insert_block_header(block_header, current_partial_mmr.peaks(), true)
            .await?;
        self.store.insert_chain_mmr_nodes(&path_nodes).await?;

        Ok(block_header)
    }
}

// UTILS
// --------------------------------------------------------------------------------------------

/// Returns a merkle path nodes for a specific block adjusted for a defined forest size.
/// This function trims the merkle path to include only the nodes that are relevant for
/// the MMR forest.
///
/// # Parameters
/// - `merkle_path`: Original merkle path.
/// - `block_num`: The block number for which the path is computed.
/// - `forest`: The target size of the forest.
fn adjust_merkle_path_for_forest(
    merkle_path: &MerklePath,
    block_num: usize,
    forest: usize,
) -> Vec<(InOrderIndex, Digest)> {
    if forest - 1 < block_num {
        panic!("Can't adjust merkle path for a forest that does not include the block number");
    }

    let rightmost_index = InOrderIndex::from_leaf_pos(forest - 1);

    let mut idx = InOrderIndex::from_leaf_pos(block_num);
    let mut path_nodes = vec![];
    for node in merkle_path.iter() {
        idx = idx.sibling();
        // Rightmost index is always the biggest value, so if the path contains any node
        // past it, we can discard it for our version of the forest
        if idx <= rightmost_index {
            path_nodes.push((idx, *node));
        }
        idx = idx.parent();
    }

    path_nodes
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

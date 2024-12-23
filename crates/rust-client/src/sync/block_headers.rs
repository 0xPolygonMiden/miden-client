use alloc::vec::Vec;

use crypto::merkle::{InOrderIndex, MmrPeaks, PartialMmr};
use miden_objects::{
    crypto::{self, merkle::MerklePath, rand::FeltRng},
    BlockHeader, Digest,
};
use tracing::warn;

use crate::{
    store::{NoteFilter, StoreError},
    Client, ClientError,
};

/// Network information management methods.
impl<R: FeltRng> Client<R> {
    /// Updates committed notes with no MMR data. These could be notes that were
    /// imported with an inclusion proof, but its block header isn't tracked.
    pub(crate) async fn update_mmr_data(&mut self) -> Result<(), ClientError> {
        let mut current_partial_mmr = self.store.build_current_partial_mmr(true).await?;

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

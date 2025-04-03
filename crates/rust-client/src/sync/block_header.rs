use alloc::{sync::Arc, vec::Vec};

use crypto::merkle::{InOrderIndex, MmrPeaks, PartialMmr};
use miden_objects::{
    Digest,
    block::{BlockHeader, BlockNumber},
    crypto::{self, merkle::MerklePath},
};
use tracing::warn;

use crate::{
    Client, ClientError,
    rpc::NodeRpcClient,
    store::{ChainMmrNodeFilter, StoreError},
};

/// Maximum number of blocks the client can be behind the network for transactions and account
/// proofs to be considered valid.
pub(crate) const MAX_BLOCK_NUMBER_DELTA: u32 = 256;

/// Network information management methods.
impl Client {
    /// Attempts to retrieve the genesis block from the store. If not found,
    /// it requests it from the node and store it.
    pub async fn ensure_genesis_in_place(&mut self) -> Result<BlockHeader, ClientError> {
        let genesis = self.store.get_block_header_by_num(0.into()).await?;

        match genesis {
            Some((block, _)) => Ok(block),
            None => self.retrieve_and_store_genesis().await,
        }
    }

    /// Calls `get_block_header_by_number` requesting the genesis block and storing it
    /// in the local database.
    async fn retrieve_and_store_genesis(&mut self) -> Result<BlockHeader, ClientError> {
        let (genesis_block, _) = self
            .rpc_api
            .get_block_header_by_number(Some(BlockNumber::GENESIS), false)
            .await?;

        let blank_mmr_peaks =
            MmrPeaks::new(0, vec![]).expect("Blank MmrPeaks should not fail to instantiate");
        // We specify that we want to store the MMR data from the genesis block as we might use it
        // as an anchor for created accounts.
        self.store.insert_block_header(&genesis_block, blank_mmr_peaks, true).await?;
        Ok(genesis_block)
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Builds the current view of the chain's [`PartialMmr`]. Because we want to add all new
    /// authentication nodes that could come from applying the MMR updates, we need to track all
    /// known leaves thus far.
    ///
    /// As part of the syncing process, we add the current block number so we don't need to
    /// track it here.
    pub(crate) async fn build_current_partial_mmr(&self) -> Result<PartialMmr, ClientError> {
        let current_block_num = self.store.get_sync_height().await?;

        let tracked_nodes = self.store.get_chain_mmr_nodes(ChainMmrNodeFilter::All).await?;
        let current_peaks = self.store.get_chain_mmr_peaks_by_block_num(current_block_num).await?;

        let track_latest = if current_block_num.as_u32() != 0 {
            match self
                .store
                .get_block_header_by_num(BlockNumber::from(current_block_num.as_u32() - 1))
                .await?
            {
                Some((_, previous_block_had_notes)) => previous_block_had_notes,
                None => false,
            }
        } else {
            false
        };

        let mut current_partial_mmr =
            PartialMmr::from_parts(current_peaks, tracked_nodes, track_latest);

        let (current_block, has_client_notes) = self
            .store
            .get_block_header_by_num(current_block_num)
            .await?
            .expect("Current block should be in the store");

        current_partial_mmr.add(current_block.commitment(), has_client_notes);

        Ok(current_partial_mmr)
    }

    /// Retrieves and stores a [`BlockHeader`] by number, and stores its authentication data as
    /// well.
    ///
    /// If the store already contains MMR data for the requested block number, the request isn't
    /// done and the stored block header is returned.
    pub(crate) async fn get_and_store_authenticated_block(
        &self,
        block_num: BlockNumber,
        current_partial_mmr: &mut PartialMmr,
    ) -> Result<BlockHeader, ClientError> {
        if current_partial_mmr.is_tracked(block_num.as_usize()) {
            warn!("Current partial MMR already contains the requested data");
            let (block_header, _) = self
                .store
                .get_block_header_by_num(block_num)
                .await?
                .expect("Block header should be tracked");
            return Ok(block_header);
        }

        // Fetch the block header and MMR proof from the node
        let (block_header, path_nodes) =
            fetch_block_header(self.rpc_api.clone(), block_num, current_partial_mmr).await?;

        // Insert header and MMR nodes
        self.store
            .insert_block_header(&block_header, current_partial_mmr.peaks(), true)
            .await?;
        self.store.insert_chain_mmr_nodes(&path_nodes).await?;

        Ok(block_header)
    }

    /// Returns the epoch block for the specified block number.
    ///
    /// If the epoch block header is not stored, it will be retrieved and stored.
    pub async fn get_epoch_block(
        &mut self,
        block_num: BlockNumber,
    ) -> Result<BlockHeader, ClientError> {
        let epoch = block_num.block_epoch();
        let epoch_block_number = BlockNumber::from_epoch(epoch);

        if let Some((epoch_block, _)) =
            self.store.get_block_header_by_num(epoch_block_number).await?
        {
            return Ok(epoch_block);
        }

        if epoch_block_number == 0.into() {
            return self.ensure_genesis_in_place().await;
        }

        let mut current_partial_mmr = self.build_current_partial_mmr().await?;
        let anchor_block = self
            .get_and_store_authenticated_block(epoch_block_number, &mut current_partial_mmr)
            .await?;

        Ok(anchor_block)
    }

    /// Returns the epoch block for the latest tracked block.
    ///
    /// If the epoch block header is not stored, it will be retrieved and stored.
    pub async fn get_latest_epoch_block(&mut self) -> Result<BlockHeader, ClientError> {
        let current_block_num = self.store.get_sync_height().await?;
        self.get_epoch_block(current_block_num).await
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
pub(crate) fn adjust_merkle_path_for_forest(
    merkle_path: &MerklePath,
    block_num: BlockNumber,
    forest: usize,
) -> Vec<(InOrderIndex, Digest)> {
    assert!(
        forest > block_num.as_usize(),
        "Can't adjust merkle path for a forest that does not include the block number"
    );

    let rightmost_index = InOrderIndex::from_leaf_pos(forest - 1);

    let mut idx = InOrderIndex::from_leaf_pos(block_num.as_usize());
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

pub(crate) async fn fetch_block_header(
    rpc_api: Arc<dyn NodeRpcClient>,
    block_num: BlockNumber,
    current_partial_mmr: &mut PartialMmr,
) -> Result<(BlockHeader, Vec<(InOrderIndex, Digest)>), ClientError> {
    let (block_header, mmr_proof) = rpc_api.get_block_header_with_proof(block_num).await?;

    // Trim merkle path to keep nodes relevant to our current PartialMmr since the node's MMR
    // might be of a forest arbitrarily higher
    let path_nodes = adjust_merkle_path_for_forest(
        &mmr_proof.merkle_path,
        block_num,
        current_partial_mmr.forest(),
    );

    let merkle_path = MerklePath::new(path_nodes.iter().map(|(_, n)| *n).collect());

    current_partial_mmr
        .track(block_num.as_usize(), block_header.commitment(), &merkle_path)
        .map_err(StoreError::MmrError)?;

    Ok((block_header, path_nodes))
}

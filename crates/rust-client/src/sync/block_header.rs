use alloc::format;
use alloc::sync::Arc;
use alloc::vec::Vec;

use miden_protocol::block::{BlockHeader, BlockNumber, ValidatorConfig};
use miden_protocol::crypto::hash::rpo::Rpo256;
use miden_protocol::crypto::merkle::MerklePath;
use miden_protocol::crypto::merkle::mmr::{
    Forest,
    InOrderIndex,
    MmrError,
    MmrPath,
    MmrProof,
    PartialMmr,
};
use miden_protocol::{Felt, Word};
use tracing::warn;

use super::{block_num_from_forest, mmr_heal};
use crate::rpc::NodeRpcClient;
use crate::rpc::domain::note::ResolvedSyncNotesBlock;
use crate::store::{BlockRelevance, StoreError};
#[cfg(feature = "testing")]
use crate::test_utils::mock::MockRpcApi;
use crate::{CachedPartialMmr, Client, ClientError};

/// The number of times a sync fetches and writes the chain updates, and the number of times an
/// import writes a block header, while the write is stale because another writer changed the chain
/// data after the client read it. A prune writes once, and the next sync prunes again.
pub(crate) const MAX_STALE_WRITE_ATTEMPTS: usize = 3;

/// Network information management methods.
impl<AUTH> Client<AUTH> {
    /// Retrieves a block header by its block number from the store.
    ///
    /// Returns `None` if the block header is not found in the store.
    pub async fn get_block_header_by_num(
        &self,
        block_num: BlockNumber,
    ) -> Result<Option<(BlockHeader, BlockRelevance)>, ClientError> {
        self.store.get_block_header_by_num(block_num).await.map_err(Into::into)
    }

    /// Retrieves the block header at the current sync height from the store.
    pub async fn get_latest_block_header(&self) -> Result<BlockHeader, ClientError> {
        let sync_height = self.store.get_sync_height().await?;
        let Some((block_header, _)) = self.get_block_header_by_num(sync_height).await? else {
            return Err(ClientError::DataStoreError(miden_tx::DataStoreError::BlockNotFound(
                sync_height,
            )));
        };
        Ok(block_header)
    }

    /// Retrieves the validator configuration committed by the block header at the current sync
    /// height.
    pub async fn get_validator_config(&self) -> Result<ValidatorConfig, ClientError> {
        Ok(self.get_latest_block_header().await?.validator_config().clone())
    }

    /// Ensures that the genesis block is available. If the genesis commitment is already cached in
    /// the RPC client, returns early. Otherwise, fetches the genesis block from the node, stores
    /// it, and sets the commitment in the RPC client.
    pub async fn ensure_genesis_in_place(&mut self) -> Result<(), ClientError> {
        if self.rpc_api.has_genesis_commitment().is_some() {
            return Ok(());
        }

        let (genesis, _) = self
            .rpc_api
            .get_block_header_by_number(Some(BlockNumber::GENESIS), false)
            .await?;

        // Genesis is untracked since there are no client notes associated with it, so we fetch no
        // MMR proof and pass no nodes.
        self.store.insert_block_header(&genesis, &[], false).await?;
        self.rpc_api.set_genesis_commitment(genesis.commitment()).await?;
        Ok(())
    }

    /// Seeds local state for offline account creation and debugging without a real node.
    ///
    /// Applies default RPC limits, then either aligns the RPC genesis with an existing stored
    /// genesis, or replaces the RPC client with [`MockRpcApi`] and runs
    /// [`Self::ensure_genesis_in_place`] so genesis comes from the mock chain.
    #[cfg(feature = "testing")]
    pub async fn prepare_offline_bootstrap(&mut self) -> Result<(), ClientError> {
        let limits = self.store.get_rpc_limits().await?.unwrap_or_default();
        self.store.set_rpc_limits(limits).await?;
        self.rpc_api.set_rpc_limits(limits).await;

        if let Some((genesis, _)) = self.store.get_block_header_by_num(BlockNumber::GENESIS).await?
        {
            self.rpc_api.set_genesis_commitment(genesis.commitment()).await?;
            return Ok(());
        }

        let rpc = MockRpcApi::default();
        self.seed_protocol_config(rpc.protocol_config()).await?;
        *self.test_rpc_api() = Arc::new(rpc);
        self.ensure_genesis_in_place().await?;
        Ok(())
    }

    /// Returns the cached [`PartialMmr`] if in-memory caching is enabled and its fingerprint
    /// matches the current store state, otherwise rebuilds from the store.
    ///
    /// If the store cannot build the partial MMR because a tracked block does not have its full
    /// path, this call requests the missing nodes from the node and writes them to the store. It
    /// contacts the node and writes to the store only in that case.
    pub async fn get_current_partial_mmr(&self) -> Result<PartialMmr, ClientError> {
        if self.cache_partial_mmr_in_memory
            && let Some(ref cached) = self.partial_mmr
            && cached.sync_height == self.store.get_sync_height().await?
            && cached.tracked_blocks_hash
                == tracked_blocks_hash(
                    self.store.get_tracked_block_header_numbers().await?.into_iter(),
                )
        {
            return Ok(cached.mmr.clone());
        }
        self.read_partial_mmr_uncached().await
    }

    /// Builds the [`PartialMmr`] from the store without the cache, and repairs the store if a
    /// tracked block does not have its full path.
    pub(crate) async fn read_partial_mmr_uncached(&self) -> Result<PartialMmr, ClientError> {
        match self.store.get_current_partial_mmr().await {
            Ok(partial_mmr) => Ok(partial_mmr),
            Err(err @ StoreError::MmrError(MmrError::InconsistentPartialMmr(_))) => {
                mmr_heal::heal(&self.store, &self.rpc_api, err).await
            },
            Err(err) => Err(err.into()),
        }
    }

    /// Stores the MMR in the cache if in-memory caching is enabled.
    ///
    /// The fingerprint comes from `mmr` itself. If the store changed after `mmr` was computed, the
    /// fingerprint does not match the store and the next read builds the MMR from the store again.
    pub(crate) fn cache_partial_mmr(&mut self, mmr: PartialMmr) -> Result<(), ClientError> {
        if !self.cache_partial_mmr_in_memory {
            return Ok(());
        }

        let sync_height = block_num_from_forest(&mmr)?;
        let tracked_blocks_hash = tracked_blocks_hash(mmr.leaves().map(|(position, _)| position));
        self.partial_mmr = Some(CachedPartialMmr { sync_height, tracked_blocks_hash, mmr });
        Ok(())
    }

    /// Stores the header and the path of each note block, and tracks each block in `partial_mmr`.
    ///
    /// `blocks` come from a note sync up to the sync height of `partial_mmr`, so each path is at
    /// the forest of `partial_mmr`.
    pub(crate) async fn insert_note_blocks(
        &self,
        blocks: &[ResolvedSyncNotesBlock],
        partial_mmr: &mut PartialMmr,
    ) -> Result<(), ClientError> {
        let forest = partial_mmr.forest();
        for block in blocks {
            let block_header = block.block_header.clone();
            let block_num = block_header.block_num();
            let path = MmrPath::new(forest, block_num.as_usize(), block.mmr_path.clone());
            let mmr_proof = MmrProof::new(path, block_header.commitment());
            self.get_and_store_authenticated_block(
                block_num,
                partial_mmr,
                Some((block_header, mmr_proof)),
            )
            .await?;
        }

        Ok(())
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Stores the [`BlockHeader`] of `block_num` with its authentication data, tracks the block in
    /// `current_partial_mmr`, and returns the header.
    ///
    /// `prefetched` is a header and an MMR proof of the block that the caller already has, for
    /// example from a note sync. Without it, this call requests both from the node. If the store
    /// already contains MMR data for the block, this call makes no request and returns the stored
    /// header.
    ///
    /// If the store refuses the write because another writer changed the chain data after
    /// `current_partial_mmr` was read, this call reads the partial MMR again and adjusts the proof
    /// it holds to the new forest. It requests a new proof only if the proof is for a smaller
    /// forest. A header that the node returns must be the prefetched header.
    pub(crate) async fn get_and_store_authenticated_block(
        &self,
        block_num: BlockNumber,
        current_partial_mmr: &mut PartialMmr,
        prefetched: Option<(BlockHeader, MmrProof)>,
    ) -> Result<BlockHeader, ClientError> {
        let expected_header = prefetched.as_ref().map(|(block_header, _)| block_header.clone());
        let mut held = prefetched;
        let mut attempt = 1;
        loop {
            if current_partial_mmr.is_tracked(block_num.as_usize()) {
                warn!("Current partial MMR already contains the requested data");
                let (block_header, _) = self
                    .store
                    .get_block_header_by_num(block_num)
                    .await?
                    .ok_or(StoreError::BlockHeaderNotFound(block_num))?;
                return Ok(block_header);
            }

            let (block_header, mmr_proof) = match held.take() {
                Some(held) => held,
                None => {
                    let fetched = fetch_block_header(&self.rpc_api, block_num).await?;
                    if expected_header.as_ref().is_some_and(|expected| *expected != fetched.0) {
                        return Err(ClientError::ChainValidationError(format!(
                            "the node returned two different headers for block {block_num}"
                        )));
                    }
                    fetched
                },
            };

            let nodes = match track_proof(current_partial_mmr, &block_header, &mmr_proof) {
                Ok(nodes) => nodes,
                // The proof is for a smaller forest than the partial MMR.
                Err(MmrError::ForestOutOfBounds(..)) if attempt < MAX_STALE_WRITE_ATTEMPTS => {
                    attempt += 1;
                    continue;
                },
                // The block is after the forest of the partial MMR, so the partial MMR is older
                // than the sync height that the caller read.
                Err(MmrError::PositionNotFound(_)) if attempt < MAX_STALE_WRITE_ATTEMPTS => {
                    attempt += 1;
                    *current_partial_mmr = self.read_partial_mmr_uncached().await?;
                    held = Some((block_header, mmr_proof));
                    continue;
                },
                Err(err) => return Err(StoreError::MmrError(err).into()),
            };

            // The header and its MMR nodes must be inserted together; a header without its nodes
            // cannot be authenticated later.
            match self.store.insert_block_header(&block_header, &nodes, true).await {
                Err(StoreError::DatabaseTransientError(err))
                    if attempt < MAX_STALE_WRITE_ATTEMPTS =>
                {
                    warn!(%block_num, %err, "store refused a block header; reading the MMR again");
                    attempt += 1;
                    *current_partial_mmr = self.read_partial_mmr_uncached().await?;
                    held = Some((block_header, mmr_proof));
                },
                result => return result.map(|()| block_header).map_err(Into::into),
            }
        }
    }
}

// UTILS
// --------------------------------------------------------------------------------------------

/// Returns a merkle path nodes for a specific block adjusted for a defined forest size. This
/// function trims the merkle path to include only the nodes that are relevant for the MMR forest.
///
/// # Parameters
/// - `merkle_path`: Original merkle path.
/// - `block_num`: The block number for which the path is computed.
/// - `forest`: The target size of the forest.
pub(crate) fn adjust_merkle_path_for_forest(
    merkle_path: &MerklePath,
    block_num: BlockNumber,
    forest: Forest,
) -> Vec<(InOrderIndex, Word)> {
    let expected_path_len = forest
        .leaf_to_corresponding_tree(block_num.as_usize())
        .expect("forest includes block number") as usize;

    let mut idx = InOrderIndex::from_leaf_pos(block_num.as_usize());
    let mut path_nodes = Vec::with_capacity(expected_path_len);

    for node in merkle_path.nodes().iter().take(expected_path_len) {
        path_nodes.push((idx.sibling(), *node));
        idx = idx.parent();
    }

    path_nodes
}

fn authenticated_block_nodes(
    block_header: &BlockHeader,
    mut path_nodes: Vec<(InOrderIndex, Word)>,
) -> Vec<(InOrderIndex, Word)> {
    let mut nodes = Vec::with_capacity(path_nodes.len() + 1);
    nodes.push((
        InOrderIndex::from_leaf_pos(block_header.block_num().as_usize()),
        block_header.commitment(),
    ));
    nodes.append(&mut path_nodes);
    nodes
}

/// Requests the header of `block_num` and its MMR proof from the node, and checks that the proof is
/// for that header.
pub(crate) async fn fetch_block_header(
    rpc_api: &Arc<dyn NodeRpcClient>,
    block_num: BlockNumber,
) -> Result<(BlockHeader, MmrProof), ClientError> {
    let (block_header, mmr_proof) = rpc_api.get_block_header_with_proof(block_num).await?;

    if block_header.block_num() != block_num
        || mmr_proof.position() != block_num.as_usize()
        || mmr_proof.leaf() != block_header.commitment()
    {
        return Err(ClientError::ChainValidationError(format!(
            "the node returned a header or an MMR proof that is not for block {block_num}"
        )));
    }

    Ok((block_header, mmr_proof))
}

/// Adjusts `mmr_proof` to the forest of `partial_mmr`, tracks the block in `partial_mmr`, and
/// returns the leaf and the path nodes to store.
fn track_proof(
    partial_mmr: &mut PartialMmr,
    block_header: &BlockHeader,
    mmr_proof: &MmrProof,
) -> Result<Vec<(InOrderIndex, Word)>, MmrError> {
    let block_num = block_header.block_num();
    let mmr_proof = mmr_proof.with_forest(partial_mmr.forest())?;
    partial_mmr.track(block_num.as_usize(), block_header.commitment(), mmr_proof.merkle_path())?;

    let path_nodes =
        adjust_merkle_path_for_forest(mmr_proof.merkle_path(), block_num, partial_mmr.forest());
    Ok(authenticated_block_nodes(block_header, path_nodes))
}

/// Returns the fingerprint of a set of tracked block numbers.
fn tracked_blocks_hash(tracked_blocks: impl Iterator<Item = usize>) -> Word {
    let elements: Vec<Felt> = tracked_blocks
        .map(|n| Felt::from(u32::try_from(n).expect("block number fits in u32")))
        .collect();
    Rpo256::hash_elements(&elements)
}

#[cfg(test)]
mod tests {
    use miden_protocol::block::{BlockHeader, BlockNumber};
    use miden_protocol::crypto::merkle::MerklePath;
    use miden_protocol::crypto::merkle::mmr::{Forest, InOrderIndex, Mmr, PartialMmr};
    use miden_protocol::{Felt, Word};

    use super::{adjust_merkle_path_for_forest, authenticated_block_nodes};

    fn word(n: u64) -> Word {
        Word::new([
            Felt::new(n).expect("test value should fit into the base field"),
            Felt::new(0).expect("zero is a valid field element"),
            Felt::new(0).expect("zero is a valid field element"),
            Felt::new(0).expect("zero is a valid field element"),
        ])
    }

    #[test]
    fn adjust_merkle_path_truncates_to_forest_bounds() {
        let forest = Forest::new(5).expect("valid forest");
        // Forest 5 <=> block 4 is rightmost leaf
        let block_num = BlockNumber::from(4u32);
        let path = MerklePath::new(vec![word(1), word(2), word(3)]);

        let adjusted = adjust_merkle_path_for_forest(&path, block_num, forest);
        // Block 4 conforms a single leaf tree so it should be empty
        assert!(adjusted.is_empty());
    }

    #[test]
    #[should_panic(expected = "forest includes block number")]
    fn adjust_merkle_path_panics_for_block_beyond_forest() {
        // Forest 5 covers leaves 0..=4, so a claimed commit height of 5 has no corresponding tree
        // and the depth lookup has nothing to return. Notes whose inclusion proof claims a height
        // past the synced view must be dropped before reaching this point.
        let forest = Forest::new(5).expect("valid forest");
        let block_num = BlockNumber::from(5u32);
        let path = MerklePath::new(vec![word(1), word(2), word(3)]);

        adjust_merkle_path_for_forest(&path, block_num, forest);
    }

    #[test]
    fn adjust_merkle_path_keeps_proof_valid_for_smaller_forest() {
        // Build a proof in a larger forest and ensure truncation does not keep siblings from a
        // different tree in the smaller forest, which would invalidate the proof.
        let mut mmr = Mmr::new();
        for value in 0u64..8 {
            mmr.add(word(value)).expect("test MMR append should succeed");
        }

        let large_forest = Forest::new(8).expect("valid forest");
        let small_forest = Forest::new(5).expect("valid forest");
        let leaf_pos = 4usize;
        let block_num = BlockNumber::from(u32::try_from(leaf_pos).unwrap());

        let proof = mmr.open_at(leaf_pos, large_forest).expect("valid proof");
        let adjusted_nodes =
            adjust_merkle_path_for_forest(proof.merkle_path(), block_num, small_forest);
        let adjusted_path = MerklePath::new(adjusted_nodes.iter().map(|(_, n)| *n).collect());

        let peaks = mmr.peaks_at(small_forest).unwrap();
        let mut partial = PartialMmr::from_peaks(peaks);
        let leaf = mmr.get(leaf_pos).expect("leaf exists");

        partial
            .track(leaf_pos, leaf, &adjusted_path)
            .expect("adjusted path should verify against smaller forest peaks");
    }

    #[test]
    fn adjust_merkle_path_correct_indices() {
        // Forest 6 has trees of size 2 and 4
        let forest = Forest::new(6).expect("valid forest");
        // Block 1 is on tree with size 4 (merkle path should have 2 nodes)
        let block_num = BlockNumber::from(1u32);
        let nodes = vec![word(10), word(11), word(12), word(13)];
        let path = MerklePath::new(nodes.clone());

        let adjusted = adjust_merkle_path_for_forest(&path, block_num, forest);

        assert_eq!(adjusted.len(), 2);
        assert_eq!(adjusted[0].1, nodes[0]);
        assert_eq!(adjusted[1].1, nodes[1]);

        let mut idx = InOrderIndex::from_leaf_pos(1);
        let expected0 = idx.sibling();
        idx = idx.parent();
        let expected1 = idx.sibling();

        assert_eq!(adjusted[0].0, expected0);
        assert_eq!(adjusted[1].0, expected1);
    }

    #[test]
    fn adjust_path_limit_correct_when_siblings_in_bounds() {
        // Ensure the expected depth limit matters even when the next sibling is "in-bounds" (but
        // not part of the leaf's subtree for that forest)
        let large_leaves = 8usize;
        let small_leaves = 7usize;
        let leaf_pos = 2usize;
        let mut mmr = Mmr::new();
        for value in 0u64..large_leaves as u64 {
            mmr.add(word(value)).expect("test MMR append should succeed");
        }

        let small_forest = Forest::new(small_leaves).expect("valid forest");
        let proof = mmr
            .open_at(leaf_pos, Forest::new(large_leaves).expect("valid forest"))
            .expect("valid proof");
        let expected_depth =
            small_forest.leaf_to_corresponding_tree(leaf_pos).expect("leaf is in forest") as usize;

        // Confirm the next sibling after the expected depth is still in bounds, which would create
        // an overlong path without the depth cap.
        let mut idx = InOrderIndex::from_leaf_pos(leaf_pos);
        for _ in 0..expected_depth {
            idx = idx.parent();
        }
        let next_sibling = idx.sibling();
        let rightmost = InOrderIndex::from_leaf_pos(small_leaves - 1);
        assert!(next_sibling <= rightmost);
        assert!(proof.merkle_path().depth() as usize > expected_depth);

        let adjusted = adjust_merkle_path_for_forest(
            proof.merkle_path(),
            BlockNumber::from(u32::try_from(leaf_pos).unwrap()),
            small_forest,
        );
        assert_eq!(adjusted.len(), expected_depth);
    }

    #[test]
    fn authenticated_block_nodes_include_leaf_commitment() {
        let block_header = BlockHeader::mock(4, None, None, &[]);
        let path_nodes = vec![
            (InOrderIndex::from_leaf_pos(4).sibling(), word(10)),
            (InOrderIndex::from_leaf_pos(4).parent().sibling(), word(11)),
        ];

        let nodes = authenticated_block_nodes(&block_header, path_nodes.clone());

        assert_eq!(nodes[0], (InOrderIndex::from_leaf_pos(4), block_header.commitment()));
        assert_eq!(&nodes[1..], path_nodes.as_slice());
    }
}

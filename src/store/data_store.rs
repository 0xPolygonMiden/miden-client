use alloc::collections::BTreeSet;

use miden_objects::{
    accounts::AccountId,
    assembly::ModuleAst,
    crypto::merkle::{InOrderIndex, MerklePath, PartialMmr},
    notes::NoteId,
    transaction::{ChainMmr, InputNote, InputNotes},
    BlockHeader,
};
use miden_tx::{DataStore, DataStoreError, TransactionInputs};

use super::{sqlite_store::SqliteStore, ChainMmrNodeFilter, NoteFilter, Store};
use crate::errors::{ClientError, StoreError};

// DATA STORE
// ================================================================================================

/// Builds a [PartialMmr] with a specified forest number and a list of blocks that should be
/// authenticated.
///
/// `authenticated_blocks` cannot contain `forest`. For authenticating the last block we have,
/// the kernel extends the MMR which is why it's not needed here.
fn build_partial_mmr_with_paths<S: Store>(
    store: &S,
    forest: u32,
    authenticated_blocks: &[BlockHeader],
) -> Result<PartialMmr, DataStoreError> {
    let mut partial_mmr: PartialMmr = {
        let current_peaks = store.get_chain_mmr_peaks_by_block_num(forest)?;

        PartialMmr::from_peaks(current_peaks)
    };

    let block_nums: Vec<u32> = authenticated_blocks.iter().map(|b| b.block_num()).collect();

    let authentication_paths =
        get_authentication_path_for_blocks(store, &block_nums, partial_mmr.forest())?;

    for (header, path) in authenticated_blocks.iter().zip(authentication_paths.iter()) {
        partial_mmr
            .track(header.block_num() as usize, header.hash(), path)
            .map_err(|err| DataStoreError::InternalError(err.to_string()))?;
    }

    Ok(partial_mmr)
}

/// Retrieves all Chain MMR nodes required for authenticating the set of blocks, and then
/// constructs the path for each of them.
///
/// This method assumes `block_nums` cannot contain `forest`.
pub fn get_authentication_path_for_blocks<S: Store>(
    store: &S,
    block_nums: &[u32],
    forest: usize,
) -> Result<Vec<MerklePath>, StoreError> {
    let mut node_indices = BTreeSet::new();

    // Calculate all needed nodes indices for generating the paths
    for block_num in block_nums {
        let path_depth = mmr_merkle_path_len(*block_num as usize, forest);

        let mut idx = InOrderIndex::from_leaf_pos(*block_num as usize);

        for _ in 0..path_depth {
            node_indices.insert(idx.sibling());
            idx = idx.parent();
        }
    }

    // Get all Mmr nodes based on collected indices
    let node_indices: Vec<InOrderIndex> = node_indices.into_iter().collect();

    let filter = ChainMmrNodeFilter::List(&node_indices);
    let mmr_nodes = store.get_chain_mmr_nodes(filter)?;

    // Construct authentication paths
    let mut authentication_paths = vec![];
    for block_num in block_nums {
        let mut merkle_nodes = vec![];
        let mut idx = InOrderIndex::from_leaf_pos(*block_num as usize);

        while let Some(node) = mmr_nodes.get(&idx.sibling()) {
            merkle_nodes.push(*node);
            idx = idx.parent();
        }
        let path = MerklePath::new(merkle_nodes);
        authentication_paths.push(path);
    }

    Ok(authentication_paths)
}

/// Calculates the merkle path length for an MMR of a specific forest and a leaf index
/// `leaf_index` is a 0-indexed leaf number and `forest` is the total amount of leaves
/// in the MMR at this point.
fn mmr_merkle_path_len(leaf_index: usize, forest: usize) -> usize {
    let before = forest & leaf_index;
    let after = forest ^ before;

    after.ilog2() as usize
}

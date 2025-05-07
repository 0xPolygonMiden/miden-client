use alloc::{boxed::Box, collections::BTreeSet, sync::Arc, vec::Vec};

use miden_objects::{
    Digest, MastForest, Word,
    account::{Account, AccountId},
    block::{BlockHeader, BlockNumber},
    crypto::merkle::{InOrderIndex, MerklePath, PartialMmr},
    transaction::PartialBlockchain,
};
use miden_tx::{DataStore, DataStoreError, MastForestStore, TransactionMastStore};

use super::{PartialBlockchainFilter, Store};
use crate::store::StoreError;

// DATA STORE
// ================================================================================================

/// Wrapper structure that implements [`DataStore`] over any [`Store`].
pub(crate) struct ClientDataStore {
    /// Local database containing information about the accounts managed by this client.
    store: alloc::sync::Arc<dyn Store>,
    /// Store used to provide MAST nodes to the transaction executor.
    transaction_mast_store: Arc<TransactionMastStore>,
}

impl ClientDataStore {
    pub fn new(store: alloc::sync::Arc<dyn Store>) -> Self {
        Self {
            store,
            transaction_mast_store: Arc::new(TransactionMastStore::new()),
        }
    }

    pub fn mast_store(&self) -> Arc<TransactionMastStore> {
        self.transaction_mast_store.clone()
    }
}

#[async_trait::async_trait(?Send)]
impl DataStore for ClientDataStore {
    async fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        mut block_refs: BTreeSet<BlockNumber>,
    ) -> Result<(Account, Option<Word>, BlockHeader, PartialBlockchain), DataStoreError> {
        // Pop last block, used as reference (it does not need to be authenticated manually)
        let ref_block = block_refs.pop_last().ok_or(DataStoreError::other("Block set is empty"))?;

        // Construct Account
        let account_record = self
            .store
            .get_account(account_id)
            .await?
            .ok_or(DataStoreError::AccountNotFound(account_id))?;

        let seed = account_record.seed().copied();
        let account: Account = account_record.into();

        // If the account is new, add its anchor block to partial MMR
        if seed.is_some() {
            assert!(account.is_new());
            let anchor_block = BlockNumber::from_epoch(account_id.anchor_epoch());
            if anchor_block != ref_block {
                block_refs.insert(anchor_block);
            }
        }

        // Get header data
        let (block_header, _had_notes) = self
            .store
            .get_block_header_by_num(ref_block)
            .await?
            .ok_or(DataStoreError::BlockNotFound(ref_block))?;

        let block_headers: Vec<BlockHeader> = self
            .store
            .get_block_headers(&block_refs)
            .await?
            .into_iter()
            .map(|(header, _has_notes)| header)
            .collect();

        let partial_mmr =
            build_partial_mmr_with_paths(&self.store, ref_block.as_u32(), &block_headers).await?;

        let partial_blockchain =
            PartialBlockchain::new(partial_mmr, block_headers).map_err(|err| {
                DataStoreError::other_with_source(
                    "error creating PartialBlockchain from internal data",
                    err,
                )
            })?;

        Ok((account, seed, block_header, partial_blockchain))
    }
}

// MAST FOREST STORE
// ================================================================================================

impl MastForestStore for ClientDataStore {
    fn get(&self, procedure_hash: &Digest) -> Option<Arc<MastForest>> {
        self.transaction_mast_store.get(procedure_hash)
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Builds a [`PartialMmr`] with a specified forest number and a list of blocks that should be
/// authenticated.
///
/// `authenticated_blocks` cannot contain `forest`. For authenticating the last block we have,
/// the kernel extends the MMR which is why it's not needed here.
async fn build_partial_mmr_with_paths(
    store: &alloc::sync::Arc<dyn Store>,
    forest: u32,
    authenticated_blocks: &[BlockHeader],
) -> Result<PartialMmr, DataStoreError> {
    let mut partial_mmr: PartialMmr = {
        let current_peaks = store
            .get_partial_blockchain_peaks_by_block_num(BlockNumber::from(forest))
            .await?;

        PartialMmr::from_peaks(current_peaks)
    };

    let block_nums: Vec<BlockNumber> =
        authenticated_blocks.iter().map(BlockHeader::block_num).collect();

    let authentication_paths =
        get_authentication_path_for_blocks(store, &block_nums, partial_mmr.forest()).await?;

    for (header, path) in authenticated_blocks.iter().zip(authentication_paths.iter()) {
        partial_mmr
            .track(header.block_num().as_usize(), header.commitment(), path)
            .map_err(|err| DataStoreError::other(format!("error constructing MMR: {err}")))?;
    }

    Ok(partial_mmr)
}

/// Retrieves all Partial Blockchain nodes required for authenticating the set of blocks, and then
/// constructs the path for each of them.
///
/// This function assumes `block_nums` doesn't contain values above or equal to `forest`.
/// If there are any such values, the function will panic when calling `mmr_merkle_path_len()`.
async fn get_authentication_path_for_blocks(
    store: &alloc::sync::Arc<dyn Store>,
    block_nums: &[BlockNumber],
    forest: usize,
) -> Result<Vec<MerklePath>, StoreError> {
    let mut node_indices = BTreeSet::new();

    // Calculate all needed nodes indices for generating the paths
    for block_num in block_nums {
        let path_depth = mmr_merkle_path_len(block_num.as_usize(), forest);

        let mut idx = InOrderIndex::from_leaf_pos(block_num.as_usize());

        for _ in 0..path_depth {
            node_indices.insert(idx.sibling());
            idx = idx.parent();
        }
    }

    // Get all MMR nodes based on collected indices
    let node_indices: Vec<InOrderIndex> = node_indices.into_iter().collect();

    let filter = PartialBlockchainFilter::List(node_indices);
    let mmr_nodes = store.get_partial_blockchain_nodes(filter).await?;

    // Construct authentication paths
    let mut authentication_paths = vec![];
    for block_num in block_nums {
        let mut merkle_nodes = vec![];
        let mut idx = InOrderIndex::from_leaf_pos(block_num.as_usize());

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
    let before: usize = forest & leaf_index;
    let after = forest ^ before;

    after.ilog2() as usize
}

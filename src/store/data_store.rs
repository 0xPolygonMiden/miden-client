use crate::errors::{ClientError, StoreError};
use objects::utils::collections::BTreeSet;

use super::{chain_data::ChainMmrNodeFilter, SqliteStore};
use crypto::merkle::{InOrderIndex, MerklePath, PartialMmr};
use miden_tx::{DataStore, DataStoreError, TransactionInputs};

use objects::{
    accounts::AccountId,
    assembly::ModuleAst,
    transaction::{ChainMmr, InputNote, InputNotes},
    BlockHeader,
};

// DATA STORE
// ================================================================================================

pub struct SqliteDataStore {
    /// Local database containing information about the accounts managed by this client.
    pub(crate) store: SqliteStore,
}

impl SqliteDataStore {
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }
}

impl DataStore for SqliteDataStore {
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[objects::notes::NoteId],
    ) -> Result<TransactionInputs, DataStoreError> {
        // Construct Account
        let (account, seed) = self.store.get_account_by_id(account_id)?;

        // Get header data
        let (block_header, _had_notes) = self.store.get_block_header_by_num(block_num)?;

        let mut list_of_notes = vec![];

        let mut notes_blocks: Vec<u32> = vec![];
        for note_id in notes {
            let input_note_record = self.store.get_input_note_by_id(*note_id)?;

            let input_note: InputNote = input_note_record
                .try_into()
                .map_err(|err: ClientError| DataStoreError::InternalError(err.to_string()))?;

            list_of_notes.push(input_note.clone());

            let note_block_num = input_note.proof().origin().block_num;

            if note_block_num != block_num {
                notes_blocks.push(note_block_num);
            }
        }

        let notes_blocks: Vec<objects::BlockHeader> = self
            .store
            .get_block_headers(&notes_blocks)?
            .iter()
            .map(|(header, _has_notes)| *header)
            .collect();

        let partial_mmr = build_partial_mmr_with_paths(&self.store, block_num, &notes_blocks)?;
        let chain_mmr = ChainMmr::new(partial_mmr, notes_blocks)
            .map_err(|err| DataStoreError::InternalError(err.to_string()))?;

        let input_notes =
            InputNotes::new(list_of_notes).map_err(DataStoreError::InvalidTransactionInput)?;

        let seed = if account.is_new() { Some(seed) } else { None };

        TransactionInputs::new(account, seed, block_header, chain_mmr, input_notes)
            .map_err(DataStoreError::InvalidTransactionInput)
    }

    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        let (_, module_ast) = self.store.get_account_code_by_account_id(account_id)?;

        Ok(module_ast)
    }
}

/// Builds a [PartialMmr] with a specified forest number and a list of blocks that should be
/// authenticated.
///
/// `authenticated_blocks` cannot contain `forest`. For authenticating the last block we have,
/// the kernel extends the MMR which is why it's not needed here.
fn build_partial_mmr_with_paths(
    store: &SqliteStore,
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
pub fn get_authentication_path_for_blocks(
    store: &SqliteStore,
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

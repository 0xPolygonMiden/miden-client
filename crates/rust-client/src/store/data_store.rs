use alloc::collections::BTreeMap;

#[cfg(feature = "async")]
use alloc::boxed::Box;
use alloc::{
    collections::BTreeSet,
    string::ToString,
    vec::Vec,
};

use miden_objects::{
    accounts::AccountId, crypto::merkle::{InOrderIndex, MerklePath, PartialMmr}, notes::NoteId, transaction::{ChainMmr, InputNote, InputNotes}, BlockHeader
};
use miden_tx::{DataStore, DataStoreError, TransactionInputs};
use winter_maybe_async::*;

use super::{ChainMmrNodeFilter, InputNoteRecord, NoteFilter, NoteRecordError, Store};
use crate::store::StoreError;

// DATA STORE
// ================================================================================================

/// Wrapper structure that implements [DataStore] over any [Store].
pub(crate) struct ClientDataStore {
    /// Local database containing information about the accounts managed by this client.
    pub(crate) store: alloc::sync::Arc<dyn Store>,
}

impl ClientDataStore {
    pub fn new(store: alloc::sync::Arc<dyn Store>) -> Self {
        Self { store }
    }
}
#[maybe_async_trait]
impl DataStore for ClientDataStore {
    #[maybe_async]
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteId],
    ) -> Result<TransactionInputs, DataStoreError> {
        let input_note_records: BTreeMap<NoteId, InputNoteRecord> =
            maybe_await!(self.store.get_input_notes(NoteFilter::List(notes.to_vec())))?
                .into_iter()
                .map(|note_record| (note_record.id(), note_record))
                .collect();

        // First validate that all notes were found and can be consumed
        for note_id in notes {
            if let Some(note_record) = input_note_records.get(note_id) {
                if note_record.is_consumed() {
                    return Err(DataStoreError::NoteAlreadyConsumed(*note_id));
                }
            } else {
                return Err(DataStoreError::NoteNotFound(*note_id));
            }
        }

        // Construct Account
        let (account, seed) = maybe_await!(self.store.get_account(account_id))?;

        // Get header data
        let (block_header, _had_notes) =
            maybe_await!(self.store.get_block_header_by_num(block_num))?;

        let mut list_of_notes = vec![];
        let mut notes_blocks: Vec<u32> = vec![];

        for (_note_id, note_record) in input_note_records {
            let input_note: InputNote = note_record
                .try_into()
                .map_err(|err: NoteRecordError| DataStoreError::InternalError(err.to_string()))?;

            list_of_notes.push(input_note.clone());

            // Include block if note is authenticated
            if let Some(inclusion_proof) = input_note.proof() {
                let note_block_num = inclusion_proof.location().block_num();

                if note_block_num != block_num {
                    notes_blocks.push(note_block_num);
                }
            }
        }

        let notes_blocks: Vec<BlockHeader> =
            maybe_await!(self.store.get_block_headers(&notes_blocks))?
                .iter()
                .map(|(header, _has_notes)| *header)
                .collect();

        let partial_mmr =
            maybe_await!(build_partial_mmr_with_paths(&self.store, block_num, &notes_blocks));
        let chain_mmr = ChainMmr::new(partial_mmr?, notes_blocks)
            .map_err(|err| DataStoreError::InternalError(err.to_string()))?;

        let input_notes =
            InputNotes::new(list_of_notes).map_err(DataStoreError::InvalidTransactionInput)?;

        TransactionInputs::new(account, seed, block_header, chain_mmr, input_notes)
            .map_err(DataStoreError::InvalidTransactionInput)
    }
}

/// Builds a [PartialMmr] with a specified forest number and a list of blocks that should be
/// authenticated.
///
/// `authenticated_blocks` cannot contain `forest`. For authenticating the last block we have,
/// the kernel extends the MMR which is why it's not needed here.
#[maybe_async]
fn build_partial_mmr_with_paths(
    store: &alloc::sync::Arc<dyn Store>,
    forest: u32,
    authenticated_blocks: &[BlockHeader],
) -> Result<PartialMmr, DataStoreError> {
    let mut partial_mmr: PartialMmr = {
        let current_peaks = maybe_await!(store.get_chain_mmr_peaks_by_block_num(forest))?;

        PartialMmr::from_peaks(current_peaks)
    };

    let block_nums: Vec<u32> = authenticated_blocks.iter().map(|b| b.block_num()).collect();

    let authentication_paths =
        maybe_await!(get_authentication_path_for_blocks(store, &block_nums, partial_mmr.forest()))?;

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
/// This function assumes `block_nums` does not contain values above or equal to `forest`.
/// If there are any such values, the function will panic when calling `mmr_merkle_path_len()`.
#[maybe_async]
fn get_authentication_path_for_blocks(
    store: &alloc::sync::Arc<dyn Store>,
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

    // Get all MMR nodes based on collected indices
    let node_indices: Vec<InOrderIndex> = node_indices.into_iter().collect();

    let filter = ChainMmrNodeFilter::List(node_indices);
    let mmr_nodes = maybe_await!(store.get_chain_mmr_nodes(filter))?;

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
    let before: usize = forest & leaf_index;
    let after = forest ^ before;

    after.ilog2() as usize
}

use crypto::merkle::{Mmr, PartialMmr, MmrPeaks};
use miden_tx::{DataStore, DataStoreError};
use objects::notes::NoteInclusionProof;
use objects::transaction::ChainMmr;
use objects::AdviceInputs;
use objects::{
    accounts::{Account, AccountId},
    assembly::ModuleAst,
    notes::{NoteOrigin, RecordedNote},
    BlockHeader,
};

use super::Store;

// DATA STORE
// ================================================================================================

pub struct SqliteDataStore {
    /// Local database containing information about the accounts managed by this client.
    pub(crate) store: Store,
}

impl SqliteDataStore {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl DataStore for SqliteDataStore {
    fn get_transaction_data(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteOrigin],
    ) -> Result<
        (
            Account,
            BlockHeader,
            ChainMmr,
            Vec<RecordedNote>,
            AdviceInputs,
        ),
        DataStoreError,
    > {
        // Construct Account
        let account = self
            .store
            .get_account_by_id(account_id)
            .map_err(|_| DataStoreError::AccountNotFound(account_id))?;

        // Get header data

        let block_header = self
            .store
            .get_block_header_by_num(block_num)
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        // Get notes data

        // TODO:
        //  - To build the return list of RecordedNote: We need to get a RecordedNote for all input notes
        //  - To build the return (partial) ChainMmr: From the block numbers in each note.origin(), get the list of block headers
        //    and construct the partial Mmr
        //  - Investigate AdviceInputs (appears not to be needed)
        let notes_block_nums = notes.iter().map(|n| n.block_num).collect();
        let mmr_paths = self.store.get_chain_mmr_paths(&notes_block_nums)?;
        let block_headers_peaks = self.store.get_mmr_peaks_by_block_num(notes_block_nums)?;

        let mmr_peaks = MmrPeaks::new(notes.len(), block_headers_peaks);
        let partial_mmr = PartialMmr::from_peaks(mmr_peaks);
        let chain_mmr = ChainMmr::new(partial_mmr, /*map(block_num, block_hash) */)?;

        let mut list_of_notes = vec![];
        for note in notes {
            let path = chain_mmr.mmr().get_path(note.node_index);
            let proof = NoteInclusionProof::new(note.block_num, /*block[note.block_num].sub_hash() */, /*block[note.block_num].note_root()*/, note.node_index, path)?;

            // Do we look for the full recorded note by inclusion proof in the DB?
            // let recorded_note = self.store.get_input_note_by_proof(proof);
            // list_of_notes.push(recorded_note);
        }

        Ok((
            account,
            block_header,
            chain_mmr,
            list_of_notes,
            AdviceInputs::default(),
        ))
    }

    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        let account = self
            .store
            .get_account_stub_by_id(account_id)
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;
        let (_, module_ast) = self
            .store
            .get_account_code(account.code_root())
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        Ok(module_ast)
    }
}

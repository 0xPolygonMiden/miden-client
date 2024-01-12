use super::Store;
use crypto::merkle::PartialMmr;
use miden_tx::{DataStore, DataStoreError, TransactionInputs};

use objects::{
    accounts::AccountId,
    assembly::ModuleAst,
    transaction::{ChainMmr, InputNote, InputNotes},
};

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
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[objects::notes::NoteId],
    ) -> Result<TransactionInputs, DataStoreError> {
        // Construct Account
        let (account, seed) = self
            .store
            .get_account_by_id(account_id)
            .map_err(|_| DataStoreError::AccountNotFound(account_id))?;

        // Get header data

        let block_header = self
            .store
            .get_block_header_by_num(block_num)
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        let mut list_of_notes = vec![];

        for note_id in notes {
            let input_note_record = self
                .store
                .get_input_note_by_id(*note_id)
                .map_err(|_| DataStoreError::AccountNotFound(account_id))?;

            let input_note: InputNote = input_note_record
                .try_into()
                .map_err(|_| DataStoreError::AccountNotFound(account_id))?;
            list_of_notes.push(input_note.clone());
        }

        // TODO:
        //  - To build the return (partial) ChainMmr: From the block numbers in each note.origin(), get the list of block headers
        //    and construct the partial Mmr
        let (partial_mmr, notes_blocks) = self
            .store
            .get_partial_mmr_for_notes(block_num, &list_of_notes)
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        let chain_mmr = ChainMmr::new(partial_mmr, notes_blocks)
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        let input_notes = InputNotes::new(list_of_notes)
            .map_err(|_| DataStoreError::AccountNotFound(account_id))?;

        let seed = if account.is_new() { Some(seed) } else { None };
        let transaction_inputs =
            TransactionInputs::new(account, seed, block_header, chain_mmr, input_notes)
                .map_err(|err| println!("{}", err))
                .unwrap();

        Ok(transaction_inputs)
    }

    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        let (_, module_ast) = self
            .store
            .get_account_code_by_account_id(account_id)
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        Ok(module_ast)
    }
}

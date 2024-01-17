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

        let (block_header, block_path) =    self
            .store
            .get_block_header_by_num(block_num)
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        let mut list_of_notes = vec![];
                                    println!("hasta aca1");
        let mut notes_blocks: Vec<objects::BlockHeader> = vec![];
        for note_id in notes {
            let input_note_record = self
                .store
                .get_input_note_by_id(*note_id)
                .map_err(|_| DataStoreError::AccountNotFound(account_id))?;

            let input_note: InputNote = input_note_record
                .try_into()
                .map_err(|_| DataStoreError::AccountNotFound(account_id))?;
            list_of_notes.push(input_note.clone());

            let note_block_num = input_note.proof().origin().block_num;
            println!("note block num {}", note_block_num);
            let (note_block, _) = self
                .store
                .get_block_header_by_num(note_block_num)
                .map_err(|_| DataStoreError::AccountNotFound(account_id))?;
            notes_blocks.push(note_block);
        }

        println!("hasta aca2 {:?}", notes_blocks);

        // TODO:
        //  - To build the return (partial) ChainMmr: From the block numbers in each note.origin(), get the list of block headers
        //    and construct the partial Mmr

        // build partial mmr from the nodes - partial_mmr should be on memory as part of our store
        let mut partial_mmr: PartialMmr = {
            // we are supposed to have data by this point, so reconstruct the partial mmr
            let current_peaks = self
                .store
                .get_chain_mmr_peaks_by_block_num(block_num)
                .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

            PartialMmr::from_peaks(current_peaks)
        };


        // if let Ok((header, block_path))= self
        // .store
        // .get_block_header_by_num(block_num)
        // .map_err(|_err| DataStoreError::AccountNotFound(account_id)) {
        //     println!("header block num {}, block path {:?}", header.block_num(), block_path);
        //     partial_mmr.add((header.block_num()) as usize, header.hash(), &block_path).map_err(|err| println!("{}",err)).unwrap();
        //     println!("hasta aca3");
    
        //     println!("path : {:?}", partial_mmr.open((block_num -1) as usize).unwrap().unwrap());
        // }
        println!("hasta acavvv4");

        let chain_mmr = ChainMmr::new(partial_mmr, notes_blocks)
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        println!("hasta aca4");

        let input_notes = InputNotes::new(list_of_notes)
            .map_err(|_| DataStoreError::AccountNotFound(account_id))?;

        println!("hasta aca4");

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

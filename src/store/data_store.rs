use miden_tx::{DataStore, DataStoreError};
use objects::AdviceInputs;
use objects::{
    accounts::{Account, AccountCode, AccountId, AccountVault},
    assembly::ModuleAst,
    notes::{NoteOrigin, RecordedNote},
    BlockHeader, ChainMmr,
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
        // TODO: create a Store method that directly returns an Account struct
        let account_stub = self
            .store
            .get_account_by_id(account_id)
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;
        let (procedures, module_ast) = self
            .store
            .get_account_code(account_stub.code_root())
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        let account_code = AccountCode::from_parts(module_ast, procedures);

        let account_storage = self
            .store
            .get_account_storage(account_stub.storage_root())
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        let account_vault = self
            .store
            .get_vault_assets(account_stub.vault_root())
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;
        let account_vault = AccountVault::new(&account_vault)
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        let account = Account::new(
            account_stub.id(),
            account_vault,
            account_storage,
            account_code,
            account_stub.nonce(),
        );

        // Get header data

        //let block_header = self
        //    .store
        //    .get_block_header_by_num(block_num)
        //    .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        // Get notes data

        // TODO:
        //  - To build the return list of RecordedNote: We need to get a RecordedNote for all input notes
        //  - To build the return (partial) ChainMmr: From the block numbers in each note.origin(), get the list of block headers
        //    and construct the partial Mmr
        //  - Investigate AdviceInputs (appears not to be needed according to mock crate)
        for note in notes {
            let block_num = note.block_num;

            //let block_header = self
            //    .store
            //    .get_block_header_by_num(block_num)
            //    .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

            //let _note_root = block_header.note_root();
        }

        let notes_list = self
            .store
            .get_recorded_notes()
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        Ok((
            account,
            _,
            ChainMmr::default(),
            notes_list,
            AdviceInputs::default(),
        ))
    }

    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        let account = self
            .store
            .get_account_by_id(account_id)
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;
        let (_, module_ast) = self
            .store
            .get_account_code(account.code_root())
            .map_err(|_err| DataStoreError::AccountNotFound(account_id))?;

        Ok(module_ast)
    }
}

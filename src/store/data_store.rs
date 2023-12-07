/// NOTE: This DataStore is a WIP
/// 
use mock::mock::account;
use objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountVault},
    assembly::ModuleAst,
    crypto::{dsa::rpo_falcon512::KeyPair, merkle::MerkleStore, utils::Serializable},
    notes::{Note, NoteOrigin, NoteScript, RecordedNote},
    BlockHeader, ChainMmr, Felt, StarkField, Word,
};
use miden_tx::{DataStore, DataStoreError};


use super::Store;

/// A `DataStore` implementation for the default client store that the TransactionExecutor can use to access local state
pub struct ClientDataStore {
    pub db: Store
}

impl ClientDataStore {
    pub fn new(db: Store) -> Self {
        Self {
            db
        }
    }
}

impl DataStore for ClientDataStore {
    fn get_transaction_data(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteOrigin],
    ) -> Result<(Account, BlockHeader, ChainMmr, Vec<RecordedNote>), DataStoreError> {
        let account = self.db.get_account_by_id(account_id).map_err(|err| DataStoreError::AccountNotFound(account_id))?;
        let (_, account_code) = self.db.get_account_code(account.code_root()).map_err(|err| DataStoreError::AccountNotFound(account_id))?;
        let storage = self.db.get_account_storage(account.storage_root()).map_err(|err| DataStoreError::AccountNotFound(account_id))?;


        todo!()
    }

    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        let (_, account_code )= self.db.get_account_by_id(account_id).and_then(
            |acc| self.db.get_account_code(acc.code_root())
        ).map_err(|err| DataStoreError::AccountNotFound(account_id))?; // TODO: Improve errors on miden-base

        Ok(account_code)
    }
}

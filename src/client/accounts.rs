use super::Client;

use std::collections::BTreeMap;

use crypto::{dsa::rpo_falcon512::KeyPair, Word};
use objects::{
    accounts::{Account, AccountId, AccountStub},
    assembly::ModuleAst,
    assets::Asset,
    Digest,
};

use crate::{errors::ClientError, store::AuthInfo};

impl Client {
    // ACCOUNT INSERTION
    // --------------------------------------------------------------------------------------------

    /// Inserts a new account into the client's store.
    pub fn insert_account(
        &mut self,
        account: &Account,
        key_pair: &KeyPair,
    ) -> Result<(), ClientError> {
        self.store
            .insert_account(account, key_pair)
            .map_err(ClientError::StoreError)
    }

    // ACCOUNT DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns summary info about the accounts managed by this client.
    ///
    /// TODO: replace `AccountStub` with a more relevant structure.
    pub fn get_accounts(&self) -> Result<Vec<AccountStub>, ClientError> {
        self.store.get_accounts().map_err(|err| err.into())
    }

    /// Returns summary info about the specified account.
    pub fn get_account_by_id(&self, account_id: AccountId) -> Result<AccountStub, ClientError> {
        self.store
            .get_account_by_id(account_id)
            .map_err(|err| err.into())
    }

    /// Returns key pair structure for an Account Id.
    pub fn get_account_auth(&self, account_id: AccountId) -> Result<AuthInfo, ClientError> {
        self.store
            .get_account_auth(account_id)
            .map_err(|err| err.into())
    }

    /// Returns vault assets from a vault root.
    pub fn get_vault_assets(&self, vault_root: Digest) -> Result<Vec<Asset>, ClientError> {
        self.store
            .get_vault_assets(vault_root)
            .map_err(|err| err.into())
    }

    /// Returns account code data from a root.
    pub fn get_account_code(
        &self,
        code_root: Digest,
    ) -> Result<(Vec<Digest>, ModuleAst), ClientError> {
        self.store
            .get_account_code(code_root)
            .map_err(|err| err.into())
    }

    /// Returns account storage data from a storage root.
    pub fn get_account_storage(
        &self,
        storage_root: Digest,
    ) -> Result<BTreeMap<u64, Word>, ClientError> {
        self.store
            .get_account_storage(storage_root)
            .map_err(|err| err.into())
    }

    /// Returns historical states for the account with the specified ID.
    ///
    /// TODO: wrap `Account` in a type with additional info.
    /// TODO: consider changing the interface to support pagination.
    pub fn get_account_history(&self, _account_id: AccountId) -> Result<Vec<Account>, ClientError> {
        todo!()
    }

    /// Returns detailed information about the current state of the account with the specified ID.
    ///
    /// TODO: wrap `Account` in a type with additional info (e.g., status).
    /// TODO: consider adding `nonce` as another parameter to identify a specific account state.
    pub fn get_account_details(&self, _account_id: AccountId) -> Result<Account, ClientError> {
        todo!()
    }
}

use crate::native_code::store::Store; 

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;

use async_trait::async_trait;

// Initialize IndexedDB
#[wasm_bindgen(module = "/js/db/schema.js")]
extern "C" {
    #[wasm_bindgen(js_name = openDatabase)]
    fn setup_indexed_db() -> js_sys::Promise;
}

// WEB STORE IMPLEMENTATION
// ================================================================================================

pub struct WebStore {}

impl WebStore {
    pub async fn new() -> Result<WebStore, ()> {
        let _ = JsFuture::from(setup_indexed_db()).await;
        Ok(WebStore {})
    }
}

#[async_trait(?Send)]
impl Store for WebStore {
    // TEST FUNCTION
    async fn insert_string(
        &mut self, 
        data: String
    ) -> Result<(), ()> {
        self.insert_string(data)
    }

    // ACCOUNTS
    async fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthInfo,
    ) -> Result<(), ()> {
        self.insert_account(account, account_seed, auth_info)
    }

    async fn get_account_ids(&self) -> Result<Vec<AccountId>, ()> {
        self.get_account_ids()
    }

    async fn get_account_stubs(&self) -> Result<Vec<(AccountStub, Option<Word>)>, ()> {
        self.get_account_stubs()
    }

    async fn get_account_stub(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), ()> {
        self.get_account_stub(account_id)
    }

    async fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<(Account, Option<Word>), ()> {
        self.get_account(account_id)
    }

    async fn get_account_auth(
        &self,
        account_id: AccountId,
    ) -> Result<AuthInfo, ()> {
        self.get_account_auth(account_id)
    }
}
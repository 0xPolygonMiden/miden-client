use miden_client::account::AccountBuilder as NativeAccountBuilder;
use wasm_bindgen::prelude::*;

use crate::models::{
    account::Account, 
    account_id_anchor::AccountIdAnchor, 
    account_storage_mode::AccountStorageMode, 
    account_type::AccountType,
    word::Word
};

#[wasm_bindgen]
pub struct AccountBuilderResult{
    account: Account,
    word: Word
}

#[wasm_bindgen]
impl AccountBuilderResult {
    #[wasm_bindgen(getter)]
    pub fn account(&self) -> Account {
        self.account.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn word(&self) -> Word {
        self.word.clone()
    }
}

#[wasm_bindgen]
pub struct AccountBuilder(NativeAccountBuilder);

#[wasm_bindgen]
impl AccountBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(init_seed: Vec<u8>) -> AccountBuilder {
        let seed_array: [u8; 32] = init_seed
            .try_into()
            .map_err(|_| JsValue::from_str("Seed must be exactly 32 bytes"))?;
        AccountBuilder(NativeAccountBuilder::new(seed_array));
    }

    pub fn anchor(mut self, anchor: &AccountIdAnchor) -> Self {
        self.0 = self.0.anchor(anchor.into());
        self
    }

    #[wasm_bindgen(js_name = "accountType")]
    pub fn account_type(mut self, account_type: &AccountType) -> Self {
        self.0 = self.0.account_type(account_type.into());
        self
    }

    // TODO: AccontStorageMode as Enum?
    #[wasm_bindgen(js_name = "storageMode")]
    pub fn storage_mode(mut self, storage_mode: &AccountStorageMode) -> Self {
        self.0 = self.0.storage_mode(storage_mode.into());
        self
    }

    #[wasm_bindgen(js_name = "withComponent")]
    pub fn with_component(mut self, account_component: &AccountComponent) -> Self {
        self.0 = self.0.with_component(account_component.into());
        self
    }

    pub fn build(self) -> AccountBuilderResult {
        (account, word) = self.0.build().map_err(|err| {
            JsValue::from_str(&format!("Failed to build account: {}", err))
        })?;
        AccountBuilderResult {
            account: account.into(),
            word: word.into(),
        }
    }
}
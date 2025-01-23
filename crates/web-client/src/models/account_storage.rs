use miden_objects::account::AccountStorage as NativeAccountStorage;
use wasm_bindgen::prelude::*;

use super::rpo_digest::RpoDigest;

#[derive(Clone)]
#[wasm_bindgen]
pub struct AccountStorage(NativeAccountStorage);

#[wasm_bindgen]
impl AccountStorage {
    pub fn commitment(&self) -> RpoDigest {
        self.0.commitment().into()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeAccountStorage> for AccountStorage {
    fn from(native_account_storage: NativeAccountStorage) -> Self {
        AccountStorage(native_account_storage)
    }
}

impl From<&NativeAccountStorage> for AccountStorage {
    fn from(native_account_storage: &NativeAccountStorage) -> Self {
        AccountStorage(native_account_storage.clone())
    }
}

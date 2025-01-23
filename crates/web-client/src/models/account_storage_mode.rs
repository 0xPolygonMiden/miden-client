use miden_client::account::AccountStorageMode as NativeAccountStorageMode;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen]
pub struct AccountStorageMode(NativeAccountStorageMode);

#[wasm_bindgen]
impl AccountStorageMode {
    pub fn private() -> AccountStorageMode {
        AccountStorageMode(NativeAccountStorageMode::Private)
    }

    pub fn public() -> AccountStorageMode {
        AccountStorageMode(NativeAccountStorageMode::Public)
    }
}

// CONVERSIONS
// ================================================================================================

impl From<AccountStorageMode> for NativeAccountStorageMode {
    fn from(storage_mode: AccountStorageMode) -> Self {
        storage_mode.0
    }
}

impl From<&AccountStorageMode> for NativeAccountStorageMode {
    fn from(storage_mode: &AccountStorageMode) -> Self {
        storage_mode.0
    }
}

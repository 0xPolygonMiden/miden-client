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

    pub fn as_str(&self) -> String {
        match self.0 {
            NativeAccountStorageMode::Private => "private".to_string(),
            NativeAccountStorageMode::Public => "public".to_string(),
        }
    }

    pub fn from_str(mode: &str) -> Result<AccountStorageMode, JsValue> {
        match mode {
            "private" => Ok(AccountStorageMode::private()),
            "public" => Ok(AccountStorageMode::public()),
            _ => Err(JsValue::from_str("Invalid AccountStorageMode string")),
        }
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

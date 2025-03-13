use core::str::FromStr;

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

    #[wasm_bindgen(js_name = "tryFromStr")]
    pub fn try_from_str(s: &str) -> Result<AccountStorageMode, JsValue> {
        let mode = NativeAccountStorageMode::from_str(s)
            .map_err(|e| JsValue::from_str(&format!("Invalid AccountStorageMode string: {e:?}")))?;
        Ok(AccountStorageMode(mode))
    }

    #[wasm_bindgen(js_name = "asStr")]
    pub fn as_str(&self) -> String {
        self.0.to_string()
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

impl AccountStorageMode {
    pub fn is_public(&self) -> bool {
        self.0 == NativeAccountStorageMode::Public
    }
}

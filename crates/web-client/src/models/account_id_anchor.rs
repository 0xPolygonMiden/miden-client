use miden_objects::{
    account::AccountIdAnchor as NativeAccountIdAnchor, block::BlockHeader as NativeBlockHeader,
};
use wasm_bindgen::prelude::*;

use crate::models::block_header::BlockHeader;

#[wasm_bindgen]
pub struct AccountIdAnchor(NativeAccountIdAnchor);

#[wasm_bindgen]
impl AccountIdAnchor {
    #[wasm_bindgen(js_name = "tryFromBlockHeader")]
    pub fn try_from_block_header(block_header: &BlockHeader) -> Result<AccountIdAnchor, JsValue> {
        let native_header: NativeBlockHeader = block_header.into();

        NativeAccountIdAnchor::try_from(&native_header)
            .map(AccountIdAnchor)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeAccountIdAnchor> for AccountIdAnchor {
    fn from(native_account_id_anchor: NativeAccountIdAnchor) -> Self {
        AccountIdAnchor(native_account_id_anchor)
    }
}

impl From<&NativeAccountIdAnchor> for AccountIdAnchor {
    fn from(native_account_id_anchor: &NativeAccountIdAnchor) -> Self {
        AccountIdAnchor(*native_account_id_anchor)
    }
}

impl From<AccountIdAnchor> for NativeAccountIdAnchor {
    fn from(account_id_anchor: AccountIdAnchor) -> Self {
        account_id_anchor.0
    }
}

impl From<&AccountIdAnchor> for NativeAccountIdAnchor {
    fn from(account_id_anchor: &AccountIdAnchor) -> Self {
        account_id_anchor.0
    }
}

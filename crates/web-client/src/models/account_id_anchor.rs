use miden_objects::account::AccountIdAnchor as NativeAccountIdAnchor;
use wasm_bindgen::prelude::*;

use crate::models::block_header::BlockHeader;

#[wasm_bindgen]
pub struct AccountIdAnchor(NativeAccountIdAnchor);

#[wasm_bindgen]
impl AccountIdAnchor {
    #[wasm_bindgen(js_name = "tryFromBlockHeader")]
    pub fn try_from_block_header(block_header: &BlockHeader) -> AccountIdAnchor {
        NativeAccountIdAnchor::try_from(block_header.into()).into()
    }

    // #[wasm_bindgen(constructor)]
    // pub fn new() -> AccountIdAnchor {
    //     AccountIdAnchor(NativeAccountIdAnchor::new())
    // }

    // pub fn with_account_id(account_id: &str) -> AccountIdAnchor {
    //     AccountIdAnchor(NativeAccountIdAnchor::with_account_id(account_id))
    // }

    // pub fn with_account_id_and_anchor(account_id: &str, anchor: &str) -> AccountIdAnchor {
    //     AccountIdAnchor(NativeAccountIdAnchor::with_account_id_and_anchor(account_id, anchor))
    // }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeAccountIdAnchor> for AccountIdAnchor {
    fn from(native_account_id_anchor: NativeAccountIdAnchor) -> Self {
        AccountIdAnchor(native_account_id_anchor)
    }
}

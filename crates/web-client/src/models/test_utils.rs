use miden_objects::accounts::{
    account_id::testing::ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
    AccountId as NativeAccountId,
};
use wasm_bindgen::prelude::*;

use super::account_id::AccountId;

#[wasm_bindgen]
pub struct TestUtils;

#[wasm_bindgen]
impl TestUtils {
    pub fn create_mock_account_id() -> AccountId {
        let native_account_id: NativeAccountId =
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN.try_into().unwrap();
        native_account_id.into()
    }
}

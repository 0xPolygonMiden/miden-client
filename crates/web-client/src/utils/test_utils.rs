use miden_client::testing::account_id::ACCOUNT_ID_REGULAR_PRIVATE_ACCOUNT_UPDATABLE_CODE;
use miden_objects::account::AccountId as NativeAccountId;
use wasm_bindgen::prelude::*;

use crate::models::account_id::AccountId;

#[wasm_bindgen]
pub struct TestUtils;

#[wasm_bindgen]
impl TestUtils {
    #[wasm_bindgen(js_name = "createMockAccountId")]
    pub fn create_mock_account_id() -> AccountId {
        let native_account_id: NativeAccountId =
            ACCOUNT_ID_REGULAR_PRIVATE_ACCOUNT_UPDATABLE_CODE.try_into().unwrap();
        native_account_id.into()
    }
}

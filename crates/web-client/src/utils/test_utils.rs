use miden_client::{
    testing::account_id::ACCOUNT_ID_REGULAR_PRIVATE_ACCOUNT_UPDATABLE_CODE,
    transaction::TransactionResult as NativeTransactionResult,
};
use miden_objects::account::AccountId as NativeAccountId;
use wasm_bindgen::prelude::*;

use crate::{
    WebClient, js_error_with_context,
    models::{account_id::AccountId, transaction_result::TransactionResult},
};

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
// WEB CLIENT TESTING HELPERS
// ================================================================================================

#[cfg(feature = "testing")]
#[wasm_bindgen]
impl WebClient {
    #[wasm_bindgen(js_name = "testingApplyTransaction")]
    pub async fn testing_apply_transaction(
        &mut self,
        tx_result: TransactionResult,
    ) -> Result<(), JsValue> {
        let native_transaction_result: NativeTransactionResult = tx_result.into();

        if let Some(client) = self.get_mut_inner() {
            client
                .testing_apply_transaction(native_transaction_result)
                .await
                .map_err(|err| js_error_with_context(err, "failed to apply transaction"))?;
            Ok(())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

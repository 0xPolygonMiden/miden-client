use miden_client::transaction::{DiscardCause, TransactionStatus as NativeTransactionStatus};
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionStatus(NativeTransactionStatus);

#[wasm_bindgen]
impl TransactionStatus {
    pub fn pending() -> TransactionStatus {
        TransactionStatus(NativeTransactionStatus::Pending)
    }

    pub fn committed(block_num: u32) -> TransactionStatus {
        TransactionStatus(NativeTransactionStatus::Committed(block_num.into()))
    }

    pub fn discarded(cause: &str) -> TransactionStatus {
        let native_cause = DiscardCause::from_string(cause).expect("Invalid discard cause");

        TransactionStatus(NativeTransactionStatus::Discarded(native_cause))
    }

    #[wasm_bindgen(js_name = "isPending")]
    pub fn is_pending(&self) -> bool {
        matches!(self.0, NativeTransactionStatus::Pending)
    }

    #[wasm_bindgen(js_name = "isCommitted")]
    pub fn is_committed(&self) -> bool {
        matches!(self.0, NativeTransactionStatus::Committed(_))
    }

    #[wasm_bindgen(js_name = "isDiscarded")]
    pub fn is_discarded(&self) -> bool {
        matches!(self.0, NativeTransactionStatus::Discarded(_))
    }

    #[wasm_bindgen(js_name = "getBlockNum")]
    pub fn get_block_num(&self) -> Option<u32> {
        match self.0 {
            NativeTransactionStatus::Committed(block_num) => Some(block_num.as_u32()),
            _ => None,
        }
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeTransactionStatus> for TransactionStatus {
    fn from(native_status: NativeTransactionStatus) -> Self {
        TransactionStatus(native_status)
    }
}

impl From<&NativeTransactionStatus> for TransactionStatus {
    fn from(native_status: &NativeTransactionStatus) -> Self {
        TransactionStatus(native_status.clone())
    }
}

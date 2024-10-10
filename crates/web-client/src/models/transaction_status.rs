use miden_client::transactions::TransactionStatus as NativeTransactionStatus;
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
        TransactionStatus(NativeTransactionStatus::Committed(block_num))
    }

    pub fn is_pending(&self) -> bool {
        matches!(self.0, NativeTransactionStatus::Pending)
    }

    pub fn is_committed(&self) -> bool {
        matches!(self.0, NativeTransactionStatus::Committed(_))
    }

    pub fn get_block_num(&self) -> Option<u32> {
        match self.0 {
            NativeTransactionStatus::Committed(block_num) => Some(block_num),
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

use miden_lib::transaction::TransactionKernel as NativeTransactionKernel;
use wasm_bindgen::prelude::*;

use crate::models::assembler::Assembler;

#[wasm_bindgen]
pub struct TransactionKernel(NativeTransactionKernel);

#[wasm_bindgen]
impl TransactionKernel {
    pub fn assembler() -> Assembler {
        NativeTransactionKernel::assembler().into()
    }
}

// CONVERSIONS
// ================================================================================================

// impl From<NativeSyncSummary> for SyncSummary {
//     fn from(native_sync_summary: NativeSyncSummary) -> Self {
//         SyncSummary(native_sync_summary)
//     }
// }

use miden_client::store::TransactionFilter as NativeTransactionFilter;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionFilter(NativeTransactionFilter);

#[wasm_bindgen]
impl TransactionFilter {
    pub fn all() -> TransactionFilter {
        TransactionFilter(NativeTransactionFilter::All)
    }

    pub fn uncommitted() -> TransactionFilter {
        TransactionFilter(NativeTransactionFilter::Uncommitted)
    }
}

// CONVERSIONS
// ================================================================================================

impl From<TransactionFilter> for NativeTransactionFilter {
    fn from(filter: TransactionFilter) -> Self {
        filter.0
    }
}

impl From<&TransactionFilter> for NativeTransactionFilter {
    fn from(filter: &TransactionFilter) -> Self {
        filter.0.clone()
    }
}

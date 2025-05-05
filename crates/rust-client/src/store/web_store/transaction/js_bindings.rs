use alloc::{string::String, vec::Vec};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{js_sys, wasm_bindgen};

// Transactions IndexedDB Operations
#[wasm_bindgen(module = "/src/store/web_store/js/transactions.js")]

extern "C" {
    // GETS
    // ================================================================================================

    #[wasm_bindgen(js_name = getTransactions)]
    pub fn idxdb_get_transactions(filter: String) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertTransactionScript)]
    pub fn idxdb_insert_transaction_script(
        script_root: Vec<u8>,
        tx_script: Option<Vec<u8>>,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = upsertTransactionRecord)]
    pub fn idxdb_upsert_transaction_record(
        transaction_id: String,
        details: Vec<u8>,
        script_root: Option<Vec<u8>>,
        block_num: String,
        committed: Option<String>,
        discard_cause: Option<Vec<u8>>,
    ) -> js_sys::Promise;
}

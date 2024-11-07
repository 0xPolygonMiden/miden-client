use alloc::{string::String, vec::Vec};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

// Transactions IndexedDB Operations
#[wasm_bindgen(module = "/src/store/web_store/js/transactions.js")]

extern "C" {
    // GETS
    // ================================================================================================

    #[wasm_bindgen(js_name = getTransactions)]
    pub fn idxdb_get_transactions(filter: String) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertTransactionScript)]
    pub fn idxdb_insert_transaction_script(
        script_hash: Vec<u8>,
        tx_script: Option<Vec<u8>>,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertProvenTransactionData)]
    pub fn idxdb_insert_proven_transaction_data(
        transaction_id: String,
        account_id: String,
        init_account_state: String,
        final_account_state: String,
        input_notes: Vec<u8>,
        output_notes: Vec<u8>,
        script_hash: Option<Vec<u8>>,
        block_num: String,
        committed: Option<String>,
    ) -> js_sys::Promise;
}

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

#[wasm_bindgen(module = "/js/db/transactions.js")]
extern "C" {
    // GETS
    // ================================================================================================

    #[wasm_bindgen(js_name = getTransactions)]
    pub fn idxdb_get_transactions(
        filter: String
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertTransactionScript)]
    pub fn idxdb_insert_transaction_script(
        script_hash: Option<Vec<u8>>,
        script_program: Option<Vec<u8>>
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertProvenTransactionData)]
    pub fn idxdb_insert_proven_transaction_data(
        transaction_id: String,
        account_id: String,
        init_account_state: String,
        final_account_state: String,
        input_notes: String,
        output_notes: Vec<u8>,
        script_program: Option<Vec<u8>>,
        script_hash: Option<Vec<u8>>,
        script_inputs: Option<String>,
        block_num: String,
        committed: Option<String>
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = markTransactionsAsCommitted)]
    pub fn idxdb_mark_transactions_as_committed(
        block_num: String,
        transaction_ids: Vec<String>
    ) -> js_sys::Promise;
}
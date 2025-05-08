use alloc::{string::String, vec::Vec};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{js_sys, wasm_bindgen};

use super::flattened_vec::FlattenedU8Vec;

// Sync IndexedDB Operations
#[wasm_bindgen(module = "/src/store/web_store/js/sync.js")]

extern "C" {
    // GETS
    // ================================================================================================

    #[wasm_bindgen(js_name = getSyncHeight)]
    pub fn idxdb_get_sync_height() -> js_sys::Promise;

    #[wasm_bindgen(js_name = getNoteTags)]
    pub fn idxdb_get_note_tags() -> js_sys::Promise;

    // INSERTS
    // ================================================================================================

    #[wasm_bindgen(js_name = addNoteTag)]
    pub fn idxdb_add_note_tag(
        tag: Vec<u8>,
        source_note_id: Option<String>,
        source_account_id: Option<String>,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = applyStateSync)]
    pub fn idxdb_apply_state_sync(
        block_num: String,
        flattened_new_block_headers: FlattenedU8Vec,
        new_block_nums: Vec<String>,
        flattened_partial_blockchain_peaks: FlattenedU8Vec,
        has_client_notes: Vec<u8>,
        serialized_node_ids: Vec<String>,
        serialized_nodes: Vec<String>,
        note_tags_to_remove_as_str: Vec<String>,
    ) -> js_sys::Promise;

    // DELETES
    // ================================================================================================
    #[wasm_bindgen(js_name = removeNoteTag)]
    pub fn idxdb_remove_note_tag(
        tag: Vec<u8>,
        source_note_id: Option<String>,
        source_account_id: Option<String>,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = discardTransactions)]
    pub fn idxdb_discard_transactions(transactions: Vec<String>) -> js_sys::Promise;
}

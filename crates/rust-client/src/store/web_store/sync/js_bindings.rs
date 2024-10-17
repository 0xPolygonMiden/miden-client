use alloc::{string::String, vec::Vec};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

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
        nullifiers: Vec<String>,
        nullifier_block_nums: Vec<String>,
        block_header: Vec<u8>,
        chain_mmr_peaks: Vec<u8>,
        has_client_notes: bool,
        serialized_node_ids: Vec<String>,
        serialized_nodes: Vec<String>,
        output_note_ids: Vec<String>,
        flattened_nested_vec_output_note_inclusion_proofs: FlattenedU8Vec,
        input_note_ids: Vec<String>,
        transactions_to_commit: Vec<String>,
        transactions_to_commit_block_nums: Vec<String>,
    ) -> js_sys::Promise;

    // DELETES
    // ================================================================================================
    #[wasm_bindgen(js_name = removeNoteTag)]
    pub fn idxdb_remove_note_tag(
        tag: Vec<u8>,
        source_note_id: Option<String>,
        source_account_id: Option<String>,
    ) -> js_sys::Promise;
}

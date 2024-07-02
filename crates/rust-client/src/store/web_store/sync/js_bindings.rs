use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

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
    pub fn idxdb_add_note_tag(tags: String) -> js_sys::Promise;

    #[wasm_bindgen(js_name = applyStateSync)]
    pub fn idxdb_apply_state_sync(
        block_num: String,
        nullifiers: Vec<String>,
        nullifier_block_nums: Vec<String>,
        block_header: String,
        chain_mmr_peaks: String,
        has_client_notes: bool,
        serialized_node_ids: Vec<String>,
        serialized_nodes: Vec<String>,
        output_note_ids: Vec<String>,
        output_note_inclusion_proofs: Vec<String>,
        input_note_ids: Vec<String>,
        input_note_inclusion_proofs: Vec<String>,
        input_note_metadatas: Vec<String>,
        transactions_to_commit: Vec<String>,
        transactions_to_commit_block_nums: Vec<String>,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = updateIgnoredNotesForTag)]
    pub fn idxdb_update_ignored_notes_for_tag(tag: String) -> js_sys::Promise;
}

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

// Account IndexedDB Operations
#[wasm_bindgen(module = "/js/db/sync.js")]
extern "C" {
    // GETS
    // ================================================================================================

    #[wasm_bindgen(js_name = getSyncHeight)]
    pub fn idxdb_get_sync_height() -> js_sys::Promise;

    #[wasm_bindgen(js_name = getNoteTags)]
    pub fn idxdb_get_note_tags() -> js_sys::Promise;

    // INSERTS
    // ================================================================================================

    #[wasm_bindgen(js_name = insertNoteTag)]
    pub fn idxdb_add_note_tag(
        tags: String
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = applyStateSync)]
    pub fn idxdb_apply_state_sync(
        block_num: String,
        nullifiers: Vec<String>,
        note_ids: Vec<String>,
        inclusion_proofs: Vec<String>,
        transactions_to_commit: Vec<String>,
    ) -> js_sys::Promise;
}
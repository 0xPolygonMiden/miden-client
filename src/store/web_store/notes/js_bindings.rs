use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

// Notes IndexedDB Operations
#[wasm_bindgen(module = "/miden-wasm/js/db/notes.js")]
extern "C" {
    // GETS
    // ================================================================================================

    #[wasm_bindgen(js_name = getInputNotes)]
    pub fn idxdb_get_input_notes(status: String) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getInputNotesFromIds)]
    pub fn idxdb_get_input_notes_from_ids(note_ids: Vec<String>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getOutputNotes)]
    pub fn idxdb_get_output_notes(status: String) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getOutputNotesFromIds)]
    pub fn idxdb_get_output_notes_from_ids(note_ids: Vec<String>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getUnspentInputNoteNullifiers)]
    pub fn idxdb_get_unspent_input_note_nullifiers() -> js_sys::Promise;

    // INSERTS
    // ================================================================================================

    #[wasm_bindgen(js_name = insertInputNote)]
    pub fn idxdb_insert_input_note(
        note_id: String,
        assets: Vec<u8>,
        recipient: String,
        status: String,
        metadata: Option<String>,
        details: String,
        note_script_hash: String,
        serialized_note_script: Vec<u8>,
        inclusion_proof: Option<String>,
        serialized_created_at: String,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertOutputNote)]
    pub fn idxdb_insert_output_note(
        note_id: String,
        assets: Vec<u8>,
        recipient: String,
        status: String,
        metadata: String,
        details: Option<String>,
        note_script_hash: Option<String>,
        serialized_note_script: Option<Vec<u8>>,
        inclusion_proof: Option<String>,
        serialized_created_at: String,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = updateNoteConsumerTxId)]
    pub fn idxdb_update_note_consumer_tx_id(
        note_id: String,
        consumer_tx_id: String,
        submitted_at: String,
    ) -> js_sys::Promise;
}

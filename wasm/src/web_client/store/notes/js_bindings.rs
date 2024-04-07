use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

// Account IndexedDB Operations
#[wasm_bindgen(module = "/js/db/notes.js")]
extern "C" {
    // GETS
    // ================================================================================================

    #[wasm_bindgen(js_name = getInputNotes)]
    pub fn idxdb_get_input_notes(
        status: String
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getInputNote)]
    pub fn idxdb_get_input_note(
        note_id: String
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getOutputNotes)]
    pub fn idxdb_get_output_notes(
        status: String
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getUnpsentInputNoteNullifiers)]
    pub fn idxdb_get_unspent_input_note_nullifiers() -> js_sys::Promise;

    // INSERTS
    // ================================================================================================
    
    #[wasm_bindgen(js_name = insertInputNote)]
    pub fn idxdb_insert_input_note(
        note_id: String,
        assets: Vec<u8>,
        recipient: String,
        status: String,
        metadata: String,
        details: String,
        inclusion_proof: Option<String>
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertOutputNote)]
    pub fn idxdb_insert_output_note(
        note_id: String,
        assets: Vec<u8>,
        recipient: String,
        status: String,
        metadata: String,
        details: String,
        inclusion_proof: Option<String>
    ) -> js_sys::Promise;
}
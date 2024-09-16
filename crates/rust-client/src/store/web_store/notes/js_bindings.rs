use alloc::{string::String, vec::Vec};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

// Notes IndexedDB Operations
#[wasm_bindgen(module = "/src/store/web_store/js/notes.js")]

extern "C" {
    // GETS
    // ================================================================================================

    #[wasm_bindgen(js_name = getInputNotes)]
    pub fn idxdb_get_input_notes(states: Vec<u8>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getIgnoredOutputNotes)]
    pub fn idxdb_get_ignored_output_notes() -> js_sys::Promise;

    #[wasm_bindgen(js_name = getInputNotesFromIds)]
    pub fn idxdb_get_input_notes_from_ids(note_ids: Vec<String>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getInputNotesFromNullifiers)]
    pub fn idxdb_get_input_notes_from_nullifiers(nullifiers: Vec<String>) -> js_sys::Promise;

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
        serial_number: Vec<u8>,
        inputs: Vec<u8>,
        note_script_hash: String,
        serialized_note_script: Vec<u8>,
        nullifier: String,
        serialized_created_at: String,
        state_discriminant: u8,
        state: Vec<u8>,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertOutputNote)]
    pub fn idxdb_insert_output_note(
        note_id: String,
        assets: Vec<u8>,
        recipient: String,
        status: String,
        metadata: Vec<u8>,
        nullifier: Option<String>,
        details: Option<Vec<u8>>,
        note_script_hash: Option<String>,
        serialized_note_script: Option<Vec<u8>>,
        inclusion_proof: Option<Vec<u8>>,
        serialized_created_at: String,
        expected_height: Option<String>,
    ) -> js_sys::Promise;
}

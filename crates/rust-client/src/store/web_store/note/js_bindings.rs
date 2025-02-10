use alloc::{string::String, vec::Vec};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{js_sys, wasm_bindgen};

// Notes IndexedDB Operations
#[wasm_bindgen(module = "/src/store/web_store/js/notes.js")]

extern "C" {
    // GETS
    // ================================================================================================

    #[wasm_bindgen(js_name = getInputNotes)]
    pub fn idxdb_get_input_notes(states: Vec<u8>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getInputNotesFromIds)]
    pub fn idxdb_get_input_notes_from_ids(note_ids: Vec<String>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getInputNotesFromNullifiers)]
    pub fn idxdb_get_input_notes_from_nullifiers(nullifiers: Vec<String>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getOutputNotes)]
    pub fn idxdb_get_output_notes(states: Vec<u8>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getOutputNotesFromIds)]
    pub fn idxdb_get_output_notes_from_ids(note_ids: Vec<String>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getOutputNotesFromNullifiers)]
    pub fn idxdb_get_output_notes_from_nullifiers(nullifiers: Vec<String>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getUnspentInputNoteNullifiers)]
    pub fn idxdb_get_unspent_input_note_nullifiers() -> js_sys::Promise;

    // INSERTS
    // ================================================================================================

    #[wasm_bindgen(js_name = upsertInputNote)]
    pub fn idxdb_upsert_input_note(
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

    #[wasm_bindgen(js_name = upsertOutputNote)]
    pub fn idxdb_upsert_output_note(
        note_id: String,
        assets: Vec<u8>,
        recipient_digest: String,
        metadata: Vec<u8>,
        nullifier: Option<String>,
        expected_height: u32,
        state_discriminant: u8,
        state: Vec<u8>,
    ) -> js_sys::Promise;
}

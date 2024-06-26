use crate::store::{InputNoteRecord, NoteFilter};
use miden_objects::{utils::Serializable, Digest};
use wasm_bindgen::prelude::*;

use crate::web_client::WebClient;

#[wasm_bindgen]
impl WebClient {
    pub async fn export_note(&mut self, note_id: String) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_id = Digest::try_from(note_id)
                .map_err(|err| format!("Failed to parse input note id: {}", err))?
                .into();

            let output_note = client
                .get_output_notes(NoteFilter::Unique(note_id))
                .await
                .unwrap()
                .pop()
                .unwrap();

            // Convert output note into InputNoteRecord before exporting
            let input_note: InputNoteRecord = output_note
                .try_into()
                .map_err(|_err| format!("Can't export note with ID {}", note_id.to_hex()))?;

            let input_note_bytes = input_note.to_bytes();

            let serialized_input_note_bytes = serde_wasm_bindgen::to_value(&input_note_bytes)
                .unwrap_or_else(|_| {
                    wasm_bindgen::throw_val(JsValue::from_str("Serialization error"))
                });

            Ok(serialized_input_note_bytes)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

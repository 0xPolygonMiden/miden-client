use miden_client::store::{InputNoteRecord, NoteFilter, OutputNoteRecord};
use miden_objects::{notes::NoteId, Digest};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::*;

use crate::WebClient;

#[derive(Serialize, Deserialize)]
pub enum WebClientNoteFilter {
    All,
    Consumed,
    Committed,
    Expected,
    Processing,
}

#[wasm_bindgen]
impl WebClient {
    pub async fn get_input_notes(&mut self, filter: JsValue) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let filter: WebClientNoteFilter = from_value(filter).unwrap();
            let native_filter = match filter {
                WebClientNoteFilter::All => NoteFilter::All,
                WebClientNoteFilter::Consumed => NoteFilter::Consumed,
                WebClientNoteFilter::Committed => NoteFilter::Committed,
                WebClientNoteFilter::Expected => NoteFilter::Expected,
                WebClientNoteFilter::Processing => NoteFilter::Processing,
            };

            let notes: Vec<InputNoteRecord> = client.get_input_notes(native_filter).await.unwrap();
            let note_ids = notes.iter().map(|note| note.id().to_string()).collect::<Vec<String>>();

            serde_wasm_bindgen::to_value(&note_ids).map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_input_note(&mut self, note_id: String) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_id: NoteId = Digest::try_from(note_id)
                .map_err(|err| format!("Failed to parse input note id: {}", err))?
                .into();
            let note: InputNoteRecord = client.get_input_note(note_id).await.unwrap();

            serde_wasm_bindgen::to_value(&note.id().to_string())
                .map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_output_notes(&mut self, filter: JsValue) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let filter: WebClientNoteFilter = from_value(filter).unwrap();
            let native_filter = match filter {
                WebClientNoteFilter::All => NoteFilter::All,
                WebClientNoteFilter::Consumed => NoteFilter::Consumed,
                WebClientNoteFilter::Committed => NoteFilter::Committed,
                WebClientNoteFilter::Expected => NoteFilter::Expected,
                WebClientNoteFilter::Processing => NoteFilter::Processing,
            };

            let notes: Vec<OutputNoteRecord> =
                client.get_output_notes(native_filter).await.unwrap();
            let note_ids = notes.iter().map(|note| note.id().to_string()).collect::<Vec<String>>();

            serde_wasm_bindgen::to_value(&note_ids).map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_output_note(&mut self, note_id: String) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_id: NoteId = Digest::try_from(note_id)
                .map_err(|err| format!("Failed to parse output note id: {}", err))?
                .into();
            let note: OutputNoteRecord = client.get_output_note(note_id).await.unwrap();

            serde_wasm_bindgen::to_value(&note.id().to_string())
                .map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

use miden_client::store::OutputNoteRecord;
use miden_objects::{
    notes::{NoteId, NoteScript as NativeNoteScript},
    Digest,
};
use wasm_bindgen::prelude::*;

use super::models::note_script::NoteScript;
use crate::{
    models::{
        account_id::AccountId, consumable_note_record::ConsumableNoteRecord,
        input_note_record::InputNoteRecord, note_filter::NoteFilter,
    },
    WebClient,
};

#[wasm_bindgen]
impl WebClient {
    pub async fn get_input_notes(
        &mut self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let result = client
                .get_input_notes(filter.into())
                .await
                .map_err(|err| JsValue::from_str(&format!("Failed to get input notes: {}", err)))?;

            Ok(result.into_iter().map(|note| note.into()).collect())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_input_note(
        &mut self,
        note_id: String,
    ) -> Result<Option<InputNoteRecord>, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_id: NoteId = Digest::try_from(note_id)
                .map_err(|err| format!("Failed to parse input note id: {}", err))?
                .into();
            let result = client
                .get_input_note(note_id)
                .await
                .map_err(|err| JsValue::from_str(&format!("Failed to get input note: {}", err)))?;

            Ok(result.map(|note| note.into()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_output_notes(&mut self, filter: NoteFilter) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let notes: Vec<OutputNoteRecord> =
                client.get_output_notes(filter.into()).await.unwrap();
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
            let note: OutputNoteRecord = client.get_output_note(note_id).await.unwrap().unwrap();

            serde_wasm_bindgen::to_value(&note.id().to_string())
                .map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn compile_note_script(&mut self, script: &str) -> Result<NoteScript, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let native_note_script: NativeNoteScript = client.compile_note_script(script).unwrap();

            Ok(native_note_script.into())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn get_consumable_notes(
        &mut self,
        account_id: Option<AccountId>,
    ) -> Result<Vec<ConsumableNoteRecord>, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let native_account_id = account_id.map(|id| id.into());
            let result = client.get_consumable_notes(native_account_id).await.map_err(|err| {
                JsValue::from_str(&format!("Failed to get consumable notes: {}", err))
            })?;

            Ok(result.into_iter().map(|record| record.into()).collect())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

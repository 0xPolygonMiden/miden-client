use miden_client::store::OutputNoteRecord;
use miden_objects::{
    Digest,
    note::{NoteId, NoteScript as NativeNoteScript},
};
use wasm_bindgen::prelude::*;

use super::models::note_script::NoteScript;
use crate::{
    WebClient, js_error_with_context,
    models::{
        account_id::AccountId, consumable_note_record::ConsumableNoteRecord,
        input_note_record::InputNoteRecord, note_filter::NoteFilter,
    },
};

#[wasm_bindgen]
impl WebClient {
    #[wasm_bindgen(js_name = "getInputNotes")]
    pub async fn get_input_notes(
        &mut self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let result = client
                .get_input_notes(filter.into())
                .await
                .map_err(|err| js_error_with_context(err, "failed to get input notes"))?;
            Ok(result.into_iter().map(Into::into).collect())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    #[wasm_bindgen(js_name = "getInputNote")]
    pub async fn get_input_note(
        &mut self,
        note_id: String,
    ) -> Result<Option<InputNoteRecord>, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_id: NoteId = Digest::try_from(note_id)
                .map_err(|err| js_error_with_context(err, "failed to parse input note id"))?
                .into();
            let result = client
                .get_input_note(note_id)
                .await
                .map_err(|err| js_error_with_context(err, "failed to get input note"))?;

            Ok(result.map(Into::into))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    #[wasm_bindgen(js_name = "getOutputNotes")]
    pub async fn get_output_notes(&mut self, filter: NoteFilter) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let notes: Vec<OutputNoteRecord> = client
                .get_output_notes(filter.into())
                .await
                .map_err(|err| js_error_with_context(err, "failed to get output notes"))?;
            let note_ids = notes.iter().map(|note| note.id().to_string()).collect::<Vec<String>>();

            serde_wasm_bindgen::to_value(&note_ids).map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    #[wasm_bindgen(js_name = "getOutputNote")]
    pub async fn get_output_note(&mut self, note_id: String) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_id: NoteId = Digest::try_from(note_id)
                .map_err(|err| js_error_with_context(err, "failed to parse output note id"))?
                .into();
            let note: OutputNoteRecord = client
                .get_output_note(note_id)
                .await
                .map_err(|err| js_error_with_context(err, "failed to get output note"))?
                .ok_or_else(|| JsValue::from_str("Note not found"))?;

            serde_wasm_bindgen::to_value(&note.id().to_string())
                .map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    #[wasm_bindgen(js_name = "compileNoteScript")]
    pub fn compile_note_script(&mut self, script: &str) -> Result<NoteScript, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let native_note_script: NativeNoteScript = client
                .compile_note_script(script)
                .map_err(|err| js_error_with_context(err, "failed to compile note script"))?;

            Ok(native_note_script.into())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    #[wasm_bindgen(js_name = "getConsumableNotes")]
    pub async fn get_consumable_notes(
        &mut self,
        account_id: Option<AccountId>,
    ) -> Result<Vec<ConsumableNoteRecord>, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let native_account_id = account_id.map(Into::into);
            let result = client
                .get_consumable_notes(native_account_id)
                .await
                .map_err(|err| js_error_with_context(err, "failed to get consumable notes"))?;

            Ok(result.into_iter().map(Into::into).collect())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

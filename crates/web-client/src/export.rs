use miden_client::{store::NoteFilter, utils::Serializable};
use miden_objects::{note::NoteFile, Digest};
use wasm_bindgen::prelude::*;

use crate::WebClient;

#[derive(Clone, Debug)]
pub enum ExportType {
    Id,
    Full,
    Partial,
}

#[wasm_bindgen]
impl WebClient {
    #[wasm_bindgen(js_name = "exportNote")]
    pub async fn export_note(
        &mut self,
        note_id: String,
        export_type: String,
    ) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_id = Digest::try_from(note_id)
                .map_err(|err| JsValue::from_str(&format!("Failed to parse input note id: {err}")))?
                .into();

            let mut output_notes = client
                .get_output_notes(NoteFilter::Unique(note_id))
                .await
                .map_err(|err| JsValue::from_str(&format!("Failed to get output notes: {err}")))?;

            let output_note =
                output_notes.pop().ok_or_else(|| JsValue::from_str("No output note found"))?;

            let export_type = match export_type.as_str() {
                "Id" => ExportType::Id,
                "Full" => ExportType::Full,
                _ => ExportType::Partial,
            };

            let note_file = match export_type {
                ExportType::Id => NoteFile::NoteId(output_note.id()),
                ExportType::Full => match output_note.inclusion_proof() {
                    Some(inclusion_proof) => NoteFile::NoteWithProof(
                        output_note.clone().try_into().map_err(|err| {
                            JsValue::from_str(&format!("Failed to convert output note: {err}"))
                        })?,
                        inclusion_proof.clone(),
                    ),
                    None => return Err(JsValue::from_str("Note does not have inclusion proof")),
                },
                ExportType::Partial => NoteFile::NoteDetails {
                    details: output_note.clone().try_into().map_err(|err| {
                        JsValue::from_str(&format!("Failed to convert output note: {err}"))
                    })?,
                    after_block_num: client.get_sync_height().await.map_err(|err| {
                        JsValue::from_str(&format!("Failed to get sync height: {err}"))
                    })?,
                    tag: Some(output_note.metadata().tag()),
                },
            };

            let input_note_bytes = note_file.to_bytes();

            let serialized_input_note_bytes = serde_wasm_bindgen::to_value(&input_note_bytes)
                .map_err(|_| JsValue::from_str("Serialization error"))?;

            Ok(serialized_input_note_bytes)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    /// Retrieves the entire underlying web store and returns it as a JsValue
    ///
    /// Meant to be used in conjunction with the force_import_store method
    pub async fn export_store(&mut self) -> Result<JsValue, JsValue> {
        let store = self.store.as_ref().ok_or(JsValue::from_str("Store not initialized"))?;
        let export =
            store.export_store().await.map_err(|err| JsValue::from_str(&format!("{err}")))?;

        Ok(export)
    }
}

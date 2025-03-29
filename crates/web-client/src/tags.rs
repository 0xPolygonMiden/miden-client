use miden_objects::note::NoteTag;
use wasm_bindgen::prelude::*;

use crate::{WebClient, js_error_with_context};

#[wasm_bindgen]
impl WebClient {
    #[wasm_bindgen(js_name = "addTag")]
    pub async fn add_tag(&mut self, tag: String) -> Result<(), JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_tag_as_u32 = tag
                .parse::<u32>()
                .map_err(|err| js_error_with_context(err, "failed to parse input note tag"))?;

            let note_tag: NoteTag = note_tag_as_u32.into();
            client
                .add_note_tag(note_tag)
                .await
                .map_err(|err| js_error_with_context(err, "failed to add note tag"))?;

            Ok(())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    #[wasm_bindgen(js_name = "removeTag")]
    pub async fn remove_tag(&mut self, tag: String) -> Result<(), JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_tag_as_u32 = tag
                .parse::<u32>()
                .map_err(|err| js_error_with_context(err, "failed to parse input note tag"))?;

            let note_tag: NoteTag = note_tag_as_u32.into();
            client
                .remove_note_tag(note_tag)
                .await
                .map_err(|err| js_error_with_context(err, "failed to remove note tag"))?;

            Ok(())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    #[wasm_bindgen(js_name = "listTags")]
    pub async fn list_tags(&mut self) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let tags: Vec<NoteTag> = client
                .get_note_tags()
                .await
                .map_err(|err| js_error_with_context(err, "failed to get note tags"))?
                .into_iter()
                .map(|tag_record| tag_record.tag)
                .collect();

            // call toString() on each tag
            let result = tags.iter().map(ToString::to_string).collect::<Vec<String>>();
            serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

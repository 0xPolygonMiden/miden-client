use miden_objects::note::NoteTag;
use wasm_bindgen::prelude::*;

use crate::WebClient;

#[wasm_bindgen]
impl WebClient {
    pub async fn add_tag(&mut self, tag: String) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_tag_as_u32 = tag.parse::<u32>().unwrap();
            let note_tag: NoteTag = note_tag_as_u32.into();
            client.add_note_tag(note_tag).await.unwrap();

            Ok(JsValue::from_str("Okay, it worked"))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn remove_tag(&mut self, tag: String) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let note_tag_as_u32 = tag.parse::<u32>().unwrap();
            let note_tag: NoteTag = note_tag_as_u32.into();
            client.remove_note_tag(note_tag).await.unwrap();

            Ok(JsValue::from_str("Okay, it worked"))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn list_tags(&mut self) -> Result<JsValue, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let tags: Vec<NoteTag> = client
                .get_note_tags()
                .await
                .unwrap()
                .into_iter()
                .map(|tag_record| tag_record.tag)
                .collect();

            // call toString() on each tag
            let result = tags.iter().map(|tag| tag.to_string()).collect::<Vec<String>>();
            serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen::from_value;
use web_sys::console;

use crate::native_code::store::NativeNoteFilter;

use super::WebClient;

#[derive(Serialize, Deserialize)]
pub enum NoteFilter {
    All,
    Pending,
    Committed,
    Consumed,
}

#[wasm_bindgen]
impl WebClient {
    pub async fn get_input_notes(
        &mut self, 
        filter: JsValue
    ) -> () {
        if let Some(ref mut client) = self.get_mut_inner() {
            let filter: NoteFilter = from_value(filter).unwrap();
            let native_filter = match filter {
                NoteFilter::Pending => NativeNoteFilter::Pending,
                NoteFilter::Committed => NativeNoteFilter::Committed,
                NoteFilter::Consumed => NativeNoteFilter::Consumed,
                NoteFilter::All => NativeNoteFilter::All
            };

            let message = client.get_input_notes(native_filter).await;
            let js_value_message = JsValue::from_str(&message);
            
            // Print the message to the Chrome console
            console::log_1(&js_value_message);
        } else {
            console::error_1(&"Client not initialized".into());
        }
    }

    pub async fn get_input_note(
        &mut self,
        note_id: String
    ) -> () {
        if let Some(ref mut client) = self.get_mut_inner() {
            // let native_note_id = Digest::try_from(note_id).map_err(|err| {
            //     let error_message = err.to_string();
            //     let js_value_error_message = JsValue::from_str(&error_message);

            //     console::log_1(&js_value_error_message);
            // })
            // .into();

            let message = client.get_input_note().await;
            let js_value_message = JsValue::from_str(&message);
            
            // Print the message to the Chrome console
            console::log_1(&js_value_message);
        } else {
            console::error_1(&"Client not initialized".into());
        }
    }

    pub async fn import_input_note(
        &mut self,
        file_data: Vec<u8>
    ) -> () {
        if let Some(ref mut client) = self.get_mut_inner() {
            // let input_note_record =
            //     InputNoteRecord::read_from_bytes(&file_data).map_err(|err| err.to_string())?;
            
            let message = client.import_input_note().await;
            let js_value_message = JsValue::from_str(&message);
            
            // Print the message to the Chrome console
            console::log_1(&js_value_message);
        } else {
            console::error_1(&"Client not initialized".into());
        }
    }
}
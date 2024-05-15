use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
pub struct NewTransactionResult {
    transaction_id: String,
    created_note_ids: Vec<String>,
}

#[wasm_bindgen]
impl NewTransactionResult {
    pub fn new(
        transaction_id: String, 
        created_note_ids: Vec<String>
    ) -> NewTransactionResult {
        NewTransactionResult {
            transaction_id,
            created_note_ids
        }
    }

    #[wasm_bindgen(getter)]
    pub fn transaction_id(&self) -> String {
        self.transaction_id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn created_note_ids(&self) -> JsValue {
        JsValue::from_serde(&self.created_note_ids).unwrap()
    }
}
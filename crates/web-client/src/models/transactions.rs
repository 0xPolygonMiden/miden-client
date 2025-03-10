use wasm_bindgen::{JsValue, prelude::*};

#[wasm_bindgen]
pub struct NewSwapTransactionResult {
    transaction_id: String,
    expected_output_note_ids: Vec<String>,
    expected_partial_note_ids: Vec<String>,
    payback_note_tag: String,
}

#[wasm_bindgen]
impl NewSwapTransactionResult {
    pub fn new(
        transaction_id: String,
        expected_output_note_ids: Vec<String>,
        expected_partial_note_ids: Vec<String>,
        payback_note_tag: Option<String>,
    ) -> NewSwapTransactionResult {
        NewSwapTransactionResult {
            transaction_id,
            expected_output_note_ids,
            expected_partial_note_ids,
            payback_note_tag: payback_note_tag.unwrap_or_default(),
        }
    }

    #[wasm_bindgen(js_name = "setNoteTag")]
    pub fn set_note_tag(&mut self, payback_note_tag: String) {
        self.payback_note_tag = payback_note_tag;
    }

    #[wasm_bindgen(js_name = "transactionId")]
    pub fn transaction_id(&self) -> String {
        self.transaction_id.clone()
    }

    #[wasm_bindgen(js_name = "expectedOutputNoteIds")]
    pub fn expected_output_note_ids(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.expected_output_note_ids).unwrap()
    }

    #[wasm_bindgen(js_name = "expectedPartialNoteIds")]
    pub fn expected_partial_note_ids(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.expected_partial_note_ids).unwrap()
    }

    #[wasm_bindgen(js_name = "paybackNoteTag")]
    pub fn payback_note_tag(&self) -> String {
        self.payback_note_tag.clone()
    }
}

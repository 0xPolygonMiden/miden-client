use miden_objects::transaction::OutputNotes as NativeOutputNotes;
use wasm_bindgen::prelude::*;

use super::{output_note::OutputNote, rpo_digest::RpoDigest};

#[derive(Clone)]
#[wasm_bindgen]
pub struct OutputNotes(NativeOutputNotes);

#[wasm_bindgen]
impl OutputNotes {
    pub fn commitment(&self) -> RpoDigest {
        self.0.commitment().into()
    }

    #[wasm_bindgen(js_name = "numNotes")]
    pub fn num_notes(&self) -> u32 {
        u32::try_from(self.0.num_notes())
            .expect("only 1024 output notes is allowed per transaction")
    }

    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[wasm_bindgen(js_name = "getNote")]
    pub fn get_note(&self, index: u32) -> OutputNote {
        self.0.get_note(index as usize).into()
    }

    pub fn notes(&self) -> Vec<OutputNote> {
        self.0.iter().cloned().map(Into::into).collect()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeOutputNotes> for OutputNotes {
    fn from(native_notes: NativeOutputNotes) -> Self {
        OutputNotes(native_notes)
    }
}

impl From<&NativeOutputNotes> for OutputNotes {
    fn from(native_notes: &NativeOutputNotes) -> Self {
        OutputNotes(native_notes.clone())
    }
}

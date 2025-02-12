use miden_objects::transaction::{InputNote as NativeInputNote, InputNotes as NativeInputNotes};
use wasm_bindgen::prelude::*;

use super::{input_note::InputNote, rpo_digest::RpoDigest};

#[derive(Clone)]
#[wasm_bindgen]
pub struct InputNotes(NativeInputNotes<NativeInputNote>);

#[wasm_bindgen]
impl InputNotes {
    pub fn commitment(&self) -> RpoDigest {
        self.0.commitment().into()
    }

    pub fn num_notes(&self) -> u8 {
        u8::try_from(self.0.num_notes()).expect("only 256 input notes is allowed")
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get_note(&self, index: u8) -> InputNote {
        self.0.get_note(index as usize).into()
    }

    // TODO: iter() ?

    // TODO: into_vec() ?
}

// CONVERSIONS
// ================================================================================================

impl From<NativeInputNotes<NativeInputNote>> for InputNotes {
    fn from(native_notes: NativeInputNotes<NativeInputNote>) -> Self {
        InputNotes(native_notes)
    }
}

impl From<&NativeInputNotes<NativeInputNote>> for InputNotes {
    fn from(native_notes: &NativeInputNotes<NativeInputNote>) -> Self {
        InputNotes(native_notes.clone())
    }
}

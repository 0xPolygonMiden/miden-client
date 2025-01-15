use miden_objects::{
    notes::{
        NoteInputs as NativeNoteInputs, NoteRecipient as NativeNoteRecipient,
        NoteScript as NativeNoteScript,
    },
    Word as NativeWord,
};
use wasm_bindgen::prelude::*;

use super::{note_inputs::NoteInputs, note_script::NoteScript, rpo_digest::RpoDigest, word::Word};

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteRecipient(NativeNoteRecipient);

#[wasm_bindgen]
impl NoteRecipient {
    #[wasm_bindgen(constructor)]
    pub fn new(serial_num: &Word, note_script: &NoteScript, inputs: &NoteInputs) -> NoteRecipient {
        let native_serial_num: NativeWord = serial_num.into();
        let native_note_script: NativeNoteScript = note_script.into();
        let native_note_inputs: NativeNoteInputs = inputs.into();
        let native_note_recipient =
            NativeNoteRecipient::new(native_serial_num, native_note_script, native_note_inputs);

        NoteRecipient(native_note_recipient)
    }

    pub fn digest(&self) -> RpoDigest {
        self.0.digest().into()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeNoteRecipient> for NoteRecipient {
    fn from(native_note_recipient: NativeNoteRecipient) -> Self {
        NoteRecipient(native_note_recipient)
    }
}

impl From<&NativeNoteRecipient> for NoteRecipient {
    fn from(native_note_recipient: &NativeNoteRecipient) -> Self {
        NoteRecipient(native_note_recipient.clone())
    }
}

impl From<NoteRecipient> for NativeNoteRecipient {
    fn from(note_recipient: NoteRecipient) -> Self {
        note_recipient.0
    }
}

impl From<&NoteRecipient> for NativeNoteRecipient {
    fn from(note_recipient: &NoteRecipient) -> Self {
        note_recipient.0.clone()
    }
}

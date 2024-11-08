use miden_objects::{
    crypto::rand::{FeltRng, RpoRandomCoin},
    notes::{
        NoteInputs as NativeNoteInputs, NoteRecipient as NativeNoteRecipient,
        NoteScript as NativeNoteScript,
    },
};
use wasm_bindgen::prelude::*;

use super::{note_inputs::NoteInputs, note_script::NoteScript, rpo_digest::RpoDigest};

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteRecipient(NativeNoteRecipient);

#[wasm_bindgen]
impl NoteRecipient {
    #[wasm_bindgen(constructor)]
    pub fn new(note_script: &NoteScript, inputs: &NoteInputs) -> NoteRecipient {
        let mut random_coin = RpoRandomCoin::new(Default::default());
        let serial_num = random_coin.draw_word();
        let native_note_script: NativeNoteScript = note_script.into();
        let native_note_inputs: NativeNoteInputs = inputs.into();
        let native_note_recipient =
            NativeNoteRecipient::new(serial_num, native_note_script, native_note_inputs);

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

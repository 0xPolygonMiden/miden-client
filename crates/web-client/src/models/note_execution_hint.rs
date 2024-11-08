use miden_objects::notes::NoteExecutionHint as NativeNoteExecutionHint;
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy)]
#[wasm_bindgen]
pub struct NoteExecutionHint(NativeNoteExecutionHint);

#[wasm_bindgen]
impl NoteExecutionHint {
    pub fn none() -> NoteExecutionHint {
        NoteExecutionHint(NativeNoteExecutionHint::None)
    }

    pub fn always() -> NoteExecutionHint {
        NoteExecutionHint(NativeNoteExecutionHint::Always)
    }

    pub fn after_block(block_num: u32) -> NoteExecutionHint {
        NoteExecutionHint(NativeNoteExecutionHint::after_block(block_num))
    }

    pub fn on_block_slot(epoch_len: u8, slot_len: u8, slot_offset: u8) -> NoteExecutionHint {
        NoteExecutionHint(NativeNoteExecutionHint::on_block_slot(epoch_len, slot_len, slot_offset))
    }

    pub fn from_parts(tag: u8, payload: u32) -> NoteExecutionHint {
        NoteExecutionHint(NativeNoteExecutionHint::from_parts(tag, payload).unwrap())
    }

    pub fn can_be_consumed(&self, block_num: u32) -> bool {
        self.0.can_be_consumed(block_num).unwrap()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NoteExecutionHint> for NativeNoteExecutionHint {
    fn from(note_execution_hint: NoteExecutionHint) -> Self {
        note_execution_hint.0
    }
}

impl From<&NoteExecutionHint> for NativeNoteExecutionHint {
    fn from(note_execution_hint: &NoteExecutionHint) -> Self {
        note_execution_hint.0
    }
}

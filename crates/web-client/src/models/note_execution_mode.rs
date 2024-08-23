use miden_objects::notes::NoteExecutionMode as NativeNoteExecutionMode;
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy)]
#[wasm_bindgen]
pub struct NoteExecutionMode(NativeNoteExecutionMode);

#[wasm_bindgen]
impl NoteExecutionMode {
    pub fn new_local() -> NoteExecutionMode {
        NoteExecutionMode(NativeNoteExecutionMode::Local)
    }

    pub fn new_network() -> NoteExecutionMode {
        NoteExecutionMode(NativeNoteExecutionMode::Network)
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        let note_execution_mode_as_str = match self.0 {
            NativeNoteExecutionMode::Local => "Local",
            NativeNoteExecutionMode::Network => "Network",
        };
        note_execution_mode_as_str.to_string()
    }
}

// Conversions

impl From<NativeNoteExecutionMode> for NoteExecutionMode {
    fn from(native_note_execution_mode: NativeNoteExecutionMode) -> Self {
        NoteExecutionMode(native_note_execution_mode)
    }
}

impl From<&NativeNoteExecutionMode> for NoteExecutionMode {
    fn from(native_note_execution_mode: &NativeNoteExecutionMode) -> Self {
        NoteExecutionMode(*native_note_execution_mode)
    }
}

impl From<NoteExecutionMode> for NativeNoteExecutionMode {
    fn from(note_execution_mode: NoteExecutionMode) -> Self {
        note_execution_mode.0
    }
}

impl From<&NoteExecutionMode> for NativeNoteExecutionMode {
    fn from(note_execution_mode: &NoteExecutionMode) -> Self {
        note_execution_mode.0
    }
}

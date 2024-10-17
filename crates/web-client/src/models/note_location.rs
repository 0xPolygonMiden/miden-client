use miden_objects::notes::NoteLocation as NativeNoteLocation;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteLocation(NativeNoteLocation);

#[wasm_bindgen]
impl NoteLocation {
    pub fn block_num(&self) -> u32 {
        self.0.block_num()
    }

    pub fn node_index_in_block(&self) -> u16 {
        self.0.node_index_in_block()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeNoteLocation> for NoteLocation {
    fn from(native_location: NativeNoteLocation) -> Self {
        NoteLocation(native_location)
    }
}

impl From<&NativeNoteLocation> for NoteLocation {
    fn from(native_location: &NativeNoteLocation) -> Self {
        NoteLocation(native_location.clone())
    }
}

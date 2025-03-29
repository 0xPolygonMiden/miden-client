use miden_objects::note::NoteLocation as NativeNoteLocation;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteLocation(NativeNoteLocation);

#[wasm_bindgen]
impl NoteLocation {
    #[wasm_bindgen(js_name = "blockNum")]
    pub fn block_num(&self) -> u32 {
        self.0.block_num().as_u32()
    }

    #[wasm_bindgen(js_name = "nodeIndexInBlock")]
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

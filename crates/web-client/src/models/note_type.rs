use miden_objects::note::NoteType as NativeNoteType;
use wasm_bindgen::prelude::*;
//use wasm_bindgen_futures::js_sys::Uint8Array;

//use crate::utils::{deserialize_from_uint8array, serialize_to_uint8array};

// Keep these masks in sync with `miden-lib/asm/miden/kernels/tx/tx.masm`
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum NoteType {
    /// Notes with this type have only their hash published to the network.
    Private = 0b10,

    /// Notes with this type are shared with the network encrypted.
    Encrypted = 0b11,

    /// Notes with this type are fully shared with the network.
    Public = 0b01,
}

impl From<NativeNoteType> for NoteType {
    fn from(value: NativeNoteType) -> Self {
        match value {
            NativeNoteType::Private => NoteType::Private,
            NativeNoteType::Public => NoteType::Public,
            NativeNoteType::Encrypted => NoteType::Encrypted,
        }
    }
}

impl From<&NativeNoteType> for NoteType {
    fn from(value: &NativeNoteType) -> Self {
        match *value {
            NativeNoteType::Private => NoteType::Private,
            NativeNoteType::Public => NoteType::Public,
            NativeNoteType::Encrypted => NoteType::Encrypted,
        }
    }
}

impl From<NoteType> for NativeNoteType {
    fn from(value: NoteType) -> Self {
        match value {
            NoteType::Private => NativeNoteType::Private,
            NoteType::Public => NativeNoteType::Public,
            NoteType::Encrypted => NativeNoteType::Encrypted,
        }
    }
}

impl From<&NoteType> for NativeNoteType {
    fn from(value: &NoteType) -> Self {
        match *value {
            NoteType::Private => NativeNoteType::Private,
            NoteType::Public => NativeNoteType::Public,
            NoteType::Encrypted => NativeNoteType::Encrypted,
        }
    }
}

// #[derive(Clone, Copy)]
// #[wasm_bindgen]
// pub struct NoteType(NativeNoteType);

// #[wasm_bindgen]
// impl NoteType {
//     pub fn private() -> NoteType {
//         NoteType(NativeNoteType::Private)
//     }

//     pub fn public() -> NoteType {
//         NoteType(NativeNoteType::Public)
//     }

//     pub fn encrypted() -> NoteType {
//         NoteType(NativeNoteType::Encrypted)
//     }

//     pub fn serialize(&self) -> Uint8Array {
//         serialize_to_uint8array(&self.0)
//     }

//     pub fn deserialize(bytes: &Uint8Array) -> Result<NoteType, JsValue> {
//         deserialize_from_uint8array::<NativeNoteType>(bytes).map(NoteType)
//     }
// }

// // CONVERSIONS
// // ================================================================================================

// impl From<NativeNoteType> for NoteType {
//     fn from(native_note_type: NativeNoteType) -> Self {
//         NoteType(native_note_type)
//     }
// }

// impl From<&NativeNoteType> for NoteType {
//     fn from(native_note_type: &NativeNoteType) -> Self {
//         NoteType(*native_note_type)
//     }
// }

// impl From<NoteType> for NativeNoteType {
//     fn from(note_type: NoteType) -> Self {
//         note_type.0
//     }
// }

// impl From<&NoteType> for NativeNoteType {
//     fn from(note_type: &NoteType) -> Self {
//         note_type.0
//     }
// }

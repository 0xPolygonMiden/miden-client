use miden_client::utils::{Deserializable, Serializable};
use miden_objects::utils::SliceReader;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::Uint8Array;

pub mod assembler_utils;

#[cfg(feature = "testing")]
pub mod test_utils;

/// Serializes any value that implements `Serializable` into a `Uint8Array`.
pub fn serialize_to_uint8array<T: Serializable>(value: &T) -> Uint8Array {
    let mut buffer = Vec::new();
    // Call the trait method to write into the buffer.
    value.write_into(&mut buffer);
    Uint8Array::from(&buffer[..])
}

/// Deserializes a `Uint8Array` into any type that implements `Deserializable`.
pub fn deserialize_from_uint8array<T: Deserializable>(bytes: &Uint8Array) -> Result<T, JsValue> {
    let vec = bytes.to_vec();
    let mut reader = SliceReader::new(&vec);
    T::read_from(&mut reader)
        .map_err(|e| JsValue::from_str(&format!("Deserialization error: {e:?}")))
}

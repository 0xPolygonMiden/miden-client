use miden_client::account::StorageMap as NativeStorageMap;
use wasm_bindgen::prelude::*;

// use crate::{
//     models::{account_id::AccountId, note_id::NoteId, transaction_id::TransactionId},
//     utils::{deserialize_from_uint8array, serialize_to_uint8array},
// };

#[wasm_bindgen]
pub struct StorageMap(NativeStorageMap);

#[wasm_bindgen]
impl StorageSlot {
    #[wasm_bindgen(constructor)]
    pub fn new() -> StorageMap {
        StorageMap(NativeStorageMap::new())
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeStorageMap> for StorageMap {
    fn from(native_storage_map: NativeStorageMap) -> Self {
        StorageMap(native_storage_map)
    }
}

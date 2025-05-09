use miden_client::account::StorageSlot as NativeStorageSlot;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct StorageSlot(NativeStorageSlot);

#[wasm_bindgen]
impl StorageSlot {
    #[wasm_bindgen(js_name = "emptyValue")]
    pub fn empty_value() -> StorageSlot {
        self.0.empty_value().into()
    }

    pub fn map(&storage_map: &StorageMap) -> StorageSlot {
        self.0.map(storage_map).into()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeStorageSlot> for StorageSlot {
    fn from(native_storage_slot: NativeStorageSlot) -> Self {
        StorageSlot(native_storage_slot)
    }
}

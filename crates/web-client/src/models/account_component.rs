use miden_objects::account::{AccountComponent as NativeAccountComponent, StorageSlot as NativeStorageSlot};
use wasm_bindgen::prelude::*;

use crate::models::{assembler::Assembler, storage_slot::StorageSlot};

#[wasm_bindgen]
pub struct AccountComponent(NativeAccountComponent);

#[wasm_bindgen]
impl AccountComponent {
    pub fn compile(
        account_code: &str,
        assembler: &Assembler,
        storage_slots: Vec<StorageSlot>,
    ) -> Result<AccountComponent, JsValue> {
        let native_slots: Vec<NativeStorageSlot> = storage_slots
            .into_iter()
            .map(Into::into)
            .collect();

        NativeAccountComponent::compile(
            account_code,
            assembler.into(),
            native_slots,
        )
        .map(|native| AccountComponent(native))
        .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "withSupportsAllTypes")]
    pub fn with_supports_all_types(mut self) -> Self {
        self.0 = self.0.with_supports_all_types();
        self
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeAccountComponent> for AccountComponent {
    fn from(native_account_component: NativeAccountComponent) -> Self {
        AccountComponent(native_account_component)
    }
}

impl From<&NativeAccountComponent> for AccountComponent {
    fn from(native_account_component: &NativeAccountComponent) -> Self {
        AccountComponent(native_account_component.clone())
    }
}

impl From<AccountComponent> for NativeAccountComponent {
    fn from(account_component: AccountComponent) -> Self {
        account_component.0
    }
}

impl From<&AccountComponent> for NativeAccountComponent {
    fn from(account_component: &AccountComponent) -> Self {
        account_component.0.clone()
    }
}

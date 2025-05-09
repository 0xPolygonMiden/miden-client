use miden_objects::account::AccountComponent as NativeAccountComponent;
use wasm_bindgen::prelude::*;

use crate::models::{assembler::Assembler, storage_slot::StorageSlot};

#[wasm_bindgen]
pub struct AccountComponent(NativeAccountComponent);

#[wasm_bindgen]
impl AccountComponent {
    pub fn compile(
        account_code: &str, 
        assembler: &Assembler, 
        storage_slots: Vec<StorageSlot> // TODO: StorageSlotArray?
    ) {
        NativeAccountComponent::compile(
            account_code, 
            assembler.into(), 
            storage_slots.into_iter().map(Into::into).collect()
        )
    }

    #[wasm_bindgen(js_name = "withSupportsAllTypes")]
    pub fn with_supports_all_types(mut self) -> Self {
        self.0 = self.0.with_supports_all_types();
        self
    }
}

// CONVERSIONS
// ================================================================================================

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

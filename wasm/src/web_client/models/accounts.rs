use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen::to_value;

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct SerializedAccountStub {
    id: String,
    nonce: String,
    vault_root: String,
    storage_root: String,
    code_root: String,
}

#[wasm_bindgen]
impl SerializedAccountStub {
    pub fn new(
        id: String,
        nonce: String,
        vault_root: String,
        storage_root: String,
        code_root: String
    ) -> SerializedAccountStub {
        SerializedAccountStub {
            id,
            nonce,
            vault_root,
            storage_root,
            code_root
        }
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn nonce(&self) -> String {
        self.nonce.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn vault_root(&self) -> String {
        self.vault_root.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn storage_root(&self) -> String {
        self.storage_root.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn code_root(&self) -> String {
        self.code_root.clone()
    }
}

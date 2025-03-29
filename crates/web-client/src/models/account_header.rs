use miden_objects::account::AccountHeader as NativeAccountHeader;
use wasm_bindgen::prelude::*;

use super::{account_id::AccountId, felt::Felt, rpo_digest::RpoDigest};

#[derive(Clone)]
#[wasm_bindgen]
pub struct AccountHeader(NativeAccountHeader);

#[wasm_bindgen]
impl AccountHeader {
    pub fn commitment(&self) -> RpoDigest {
        self.0.commitment().into()
    }

    pub fn id(&self) -> AccountId {
        self.0.id().into()
    }

    pub fn nonce(&self) -> Felt {
        self.0.nonce().into()
    }

    #[wasm_bindgen(js_name = "vaultCommitment")]
    pub fn vault_commitment(&self) -> RpoDigest {
        self.0.vault_root().into()
    }

    #[wasm_bindgen(js_name = "storageCommitment")]
    pub fn storage_commitment(&self) -> RpoDigest {
        self.0.storage_commitment().into()
    }

    #[wasm_bindgen(js_name = "codeCommitment")]
    pub fn code_commitment(&self) -> RpoDigest {
        self.0.code_commitment().into()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeAccountHeader> for AccountHeader {
    fn from(native_account_header: NativeAccountHeader) -> Self {
        AccountHeader(native_account_header)
    }
}

impl From<&NativeAccountHeader> for AccountHeader {
    fn from(native_account_header: &NativeAccountHeader) -> Self {
        AccountHeader(native_account_header.clone())
    }
}

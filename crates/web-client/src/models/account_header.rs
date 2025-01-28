use miden_objects::account::AccountHeader as NativeAccountHeader;
use wasm_bindgen::prelude::*;

use super::{account_id::AccountId, felt::Felt, rpo_digest::RpoDigest};

#[derive(Clone)]
#[wasm_bindgen]
pub struct AccountHeader(NativeAccountHeader);

#[wasm_bindgen]
impl AccountHeader {
    pub fn hash(&self) -> RpoDigest {
        self.0.hash().into()
    }

    pub fn id(&self) -> AccountId {
        self.0.id().into()
    }

    pub fn nonce(&self) -> Felt {
        self.0.nonce().into()
    }

    pub fn vault_commitment(&self) -> RpoDigest {
        self.0.vault_root().into()
    }

    pub fn storage_commitment(&self) -> RpoDigest {
        self.0.storage_commitment().into()
    }

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

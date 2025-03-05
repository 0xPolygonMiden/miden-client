use miden_objects::account::{Account as NativeAccount, AccountType as NativeAccountType};
use wasm_bindgen::prelude::*;

use super::{
    account_code::AccountCode, account_id::AccountId, account_storage::AccountStorage,
    asset_vault::AssetVault, felt::Felt, rpo_digest::RpoDigest,
};

#[wasm_bindgen]
pub struct Account(NativeAccount);

#[wasm_bindgen]
impl Account {
    pub fn id(&self) -> AccountId {
        self.0.id().into()
    }

    pub fn hash(&self) -> RpoDigest {
        self.0.hash().into()
    }

    pub fn nonce(&self) -> Felt {
        self.0.nonce().into()
    }

    pub fn vault(&self) -> AssetVault {
        self.0.vault().into()
    }

    pub fn storage(&self) -> AccountStorage {
        self.0.storage().into()
    }

    pub fn code(&self) -> AccountCode {
        self.0.code().into()
    }

    #[wasm_bindgen(js_name = "isFaucet")]
    pub fn is_faucet(&self) -> bool {
        self.0.is_faucet()
    }

    #[wasm_bindgen(js_name = "isRegularAccount")]
    pub fn is_regular_account(&self) -> bool {
        self.0.is_regular_account()
    }

    #[wasm_bindgen(js_name = "isUpdatable")]
    pub fn is_updatable(&self) -> bool {
        matches!(self.0.account_type(), NativeAccountType::RegularAccountUpdatableCode)
    }

    #[wasm_bindgen(js_name = "isPublic")]
    pub fn is_public(&self) -> bool {
        self.0.is_public()
    }

    #[wasm_bindgen(js_name = "isNew")]
    pub fn is_new(&self) -> bool {
        self.0.is_new()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeAccount> for Account {
    fn from(native_account: NativeAccount) -> Self {
        Account(native_account)
    }
}

impl From<&NativeAccount> for Account {
    fn from(native_account: &NativeAccount) -> Self {
        Account(native_account.clone())
    }
}

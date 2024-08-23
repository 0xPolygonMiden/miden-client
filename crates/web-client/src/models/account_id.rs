use miden_objects::{accounts::AccountId as NativeAccountId, Felt as NativeFelt};
use wasm_bindgen::prelude::*;

use super::felt::Felt;

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct AccountId(NativeAccountId);

#[wasm_bindgen]
impl AccountId {
    pub fn is_faucet(&self) -> bool {
        self.0.is_faucet()
    }

    pub fn is_regular_account(&self) -> bool {
        self.0.is_regular_account()
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }

    pub fn to_felt(&self) -> Felt {
        let native_felt: NativeFelt = self.0.into();
        native_felt.into()
    }
}

// Conversions

impl From<NativeAccountId> for AccountId {
    fn from(native_account_id: NativeAccountId) -> Self {
        AccountId(native_account_id)
    }
}

impl From<&NativeAccountId> for AccountId {
    fn from(native_account_id: &NativeAccountId) -> Self {
        AccountId(*native_account_id)
    }
}

impl From<AccountId> for NativeAccountId {
    fn from(account_id: AccountId) -> Self {
        account_id.0
    }
}

impl From<&AccountId> for NativeAccountId {
    fn from(account_id: &AccountId) -> Self {
        account_id.0
    }
}

impl From<AccountId> for Felt {
    fn from(account_id: AccountId) -> Self {
        let native_felt: NativeFelt = account_id.0.into();
        native_felt.into()
    }
}

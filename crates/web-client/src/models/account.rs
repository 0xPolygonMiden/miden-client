use miden_objects::{
    Word as NativeWord,
    account::{Account as NativeAccount, AccountType as NativeAccountType},
    asset::TokenSymbol,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::Uint8Array;

use crate::{
    models::{
        account_code::AccountCode, account_id::AccountId, account_storage::AccountStorage,
        asset_vault::AssetVault, felt::Felt, rpo_digest::RpoDigest,
    },
    utils::{deserialize_from_uint8array, serialize_to_uint8array},
};

#[wasm_bindgen]
pub struct Account(NativeAccount);

#[wasm_bindgen]
impl Account {
    pub fn id(&self) -> AccountId {
        self.0.id().into()
    }

    pub fn commitment(&self) -> RpoDigest {
        self.0.commitment().into()
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

    #[wasm_bindgen(js_name = "tokenSymbol")]
    pub fn token_symbol(&self) -> Result<String, JsValue> {
        if !self.is_faucet() {
            return Err(JsValue::from_str("Account is not a faucet"));
        }

        // Get the token metadata from storage slot 2
        let metadata = self
            .storage()
            .get_item(2)
            .ok_or_else(|| JsValue::from_str("Token metadata not found"))?;
        // The token symbol is the third word of the metadata
        let metadata: NativeWord = metadata.to_word().into();
        let symbol_felt = metadata[2];
        let symbol = TokenSymbol::try_from(symbol_felt)
            .map_err(|err| JsValue::from_str(&format!("Invalid token symbol: {err}")))?;
        let symbol_str = symbol.to_str();

        // Find the actual length by looking for the first non-zero digit in base-26
        let mut value = symbol_felt.as_int();
        let mut length = 0;
        while value > 0 {
            value /= 26;
            length += 1;
        }

        // Special case: if length is 0, it means the value is 0, which represents "A"
        if length == 0 {
            return Ok("A".to_string());
        }

        // The string is prefixed with 'A's, so we need to take the last 'length' characters
        Ok(symbol_str[symbol_str.len() - length..].to_string())
    }

    pub fn serialize(&self) -> Uint8Array {
        serialize_to_uint8array(&self.0)
    }

    pub fn deserialize(bytes: &Uint8Array) -> Result<Account, JsValue> {
        deserialize_from_uint8array::<NativeAccount>(bytes).map(Account)
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

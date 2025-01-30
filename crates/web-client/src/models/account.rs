use miden_objects::account::{Account as NativeAccount, AccountType as NativeAccountType};
use miden_client::utils::{Deserializable, Serializable};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::Uint8Array;
use miden_crypto::utils::SliceReader;
use serde::{Serialize, Deserialize};

use super::{
    account_code::AccountCode, account_id::AccountId, account_storage::AccountStorage,
    asset_vault::AssetVault, felt::Felt, rpo_digest::RpoDigest,
};

#[wasm_bindgen]
// #[derive(Serialize, Deserialize)]  // Use Serde for auto serialization
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

    pub fn is_faucet(&self) -> bool {
        self.0.is_faucet()
    }

    pub fn is_regular_account(&self) -> bool {
        self.0.is_regular_account()
    }

    pub fn is_updatable(&self) -> bool {
        matches!(self.0.account_type(), NativeAccountType::RegularAccountUpdatableCode)
    }

    pub fn is_public(&self) -> bool {
        self.0.is_public()
    }

    pub fn is_new(&self) -> bool {
        self.0.is_new()
    }

    pub fn serialize(&self) -> Uint8Array {
        // Estimate the size for the buffer
        let native_account = &self.0;
        let size_hint = native_account.get_size_hint();
        let mut buffer = vec![0u8; size_hint];
        native_account.write_into(&mut buffer.as_mut_slice());
        Uint8Array::from(&buffer[..])
    }

    pub fn deserialize(bytes: Uint8Array) -> Result<Account, JsValue> {
        let vec: Vec<u8> = bytes.to_vec();
        let mut reader = SliceReader::new(&vec); // Wrap with SliceReader
        let native_account = NativeAccount::read_from(&mut reader).map_err(|e| JsValue::from_str(&format!("Deserialization error: {:?}", e)));
        Ok(Account(native_account?))
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

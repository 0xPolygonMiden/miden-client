use miden_objects::asset::AssetVault as NativeAssetVault;
use wasm_bindgen::prelude::*;

use super::{account_id::AccountId, rpo_digest::RpoDigest};

#[derive(Clone)]
#[wasm_bindgen]
pub struct AssetVault(NativeAssetVault);

#[wasm_bindgen]
impl AssetVault {
    pub fn commitment(&self) -> RpoDigest {
        self.0.commitment().into()
    }

    #[wasm_bindgen(js_name = "getBalance")]
    pub fn get_balance(&self, faucet_id: &AccountId) -> u64 {
        self.0.get_balance(faucet_id.into()).unwrap()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeAssetVault> for AssetVault {
    fn from(native_asset_vault: NativeAssetVault) -> Self {
        AssetVault(native_asset_vault)
    }
}

impl From<&NativeAssetVault> for AssetVault {
    fn from(native_asset_vault: &NativeAssetVault) -> Self {
        AssetVault(native_asset_vault.clone())
    }
}

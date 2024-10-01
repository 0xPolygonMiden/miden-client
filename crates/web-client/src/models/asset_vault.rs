use miden_objects::assets::AssetVault as NativeAssetVault;
use wasm_bindgen::prelude::*;

use super::rpo_digest::RpoDigest;

#[derive(Clone)]
#[wasm_bindgen]
pub struct AssetVault(NativeAssetVault);

#[wasm_bindgen]
impl AssetVault {
    pub fn commitment(&self) -> RpoDigest {
        self.0.commitment().into()
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

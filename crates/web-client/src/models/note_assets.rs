use miden_objects::{assets::Asset as NativeAsset, notes::NoteAssets as NativeNoteAssets};
use wasm_bindgen::prelude::*;

use super::fungible_asset::FungibleAsset;

#[derive(Clone)]
#[wasm_bindgen]
pub struct NoteAssets(NativeNoteAssets);

#[wasm_bindgen]
impl NoteAssets {
    #[wasm_bindgen(constructor)]
    pub fn new(assets_array: Option<Vec<FungibleAsset>>) -> NoteAssets {
        let assets = assets_array.unwrap_or_default();
        let native_assets: Vec<NativeAsset> =
            assets.into_iter().map(|asset| asset.into()).collect();
        NoteAssets(NativeNoteAssets::new(native_assets).unwrap())
    }

    pub fn push(&mut self, asset: &FungibleAsset) {
        let _ = self.0.add_asset(asset.into());
    }
}

// Conversions

impl From<NoteAssets> for NativeNoteAssets {
    fn from(note_assets: NoteAssets) -> Self {
        note_assets.0
    }
}

impl From<&NoteAssets> for NativeNoteAssets {
    fn from(note_assets: &NoteAssets) -> Self {
        note_assets.0.clone()
    }
}

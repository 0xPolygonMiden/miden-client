use miden_objects::{asset::Asset as NativeAsset, note::NoteAssets as NativeNoteAssets};
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
        let native_assets: Vec<NativeAsset> = assets.into_iter().map(Into::into).collect();
        NoteAssets(NativeNoteAssets::new(native_assets).unwrap())
    }

    pub fn push(&mut self, asset: &FungibleAsset) {
        self.0.add_asset(asset.into()).unwrap();
    }

    pub fn assets(&self) -> Vec<FungibleAsset> {
        self.0
            .iter()
            .filter_map(|asset| {
                if asset.is_fungible() {
                    Some(asset.unwrap_fungible().into())
                } else {
                    None // TODO: Support non fungible assets
                }
            })
            .collect()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeNoteAssets> for NoteAssets {
    fn from(native_note_assets: NativeNoteAssets) -> Self {
        NoteAssets(native_note_assets)
    }
}

impl From<&NativeNoteAssets> for NoteAssets {
    fn from(native_note_assets: &NativeNoteAssets) -> Self {
        NoteAssets(native_note_assets.clone())
    }
}

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

use miden_objects::crypto::merkle::MerklePath as NativeMerklePath;
use wasm_bindgen::prelude::*;

use super::rpo_digest::RpoDigest;

#[derive(Clone)]
#[wasm_bindgen]
pub struct MerklePath(NativeMerklePath);

#[wasm_bindgen]
impl MerklePath {
    pub fn depth(&self) -> u8 {
        self.0.depth()
    }

    pub fn nodes(&self) -> Vec<RpoDigest> {
        self.0.nodes().iter().map(|node| node.into()).collect()
    }

    pub fn compute_root(&self, index: u64, node: &RpoDigest) -> RpoDigest {
        self.0.compute_root(index, node.clone().into()).unwrap().into()
    }

    pub fn verify(&self, index: u64, node: &RpoDigest, root: &RpoDigest) -> bool {
        self.0.verify(index, node.clone().into(), &root.clone().into()).is_ok()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeMerklePath> for MerklePath {
    fn from(native_path: NativeMerklePath) -> Self {
        MerklePath(native_path)
    }
}

impl From<&NativeMerklePath> for MerklePath {
    fn from(native_path: &NativeMerklePath) -> Self {
        MerklePath(native_path.clone())
    }
}

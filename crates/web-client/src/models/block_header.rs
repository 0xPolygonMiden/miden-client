use miden_objects::block::BlockHeader as NativeBlockHeader;
use wasm_bindgen::prelude::*;

use super::rpo_digest::RpoDigest;

#[derive(Clone)]
#[wasm_bindgen]
pub struct BlockHeader(NativeBlockHeader);

#[wasm_bindgen]
impl BlockHeader {
    pub fn version(&self) -> u32 {
        self.0.version()
    }

    pub fn hash(&self) -> RpoDigest {
        self.0.hash().into()
    }

    #[wasm_bindgen(js_name = "subHash")]
    pub fn sub_hash(&self) -> RpoDigest {
        self.0.sub_hash().into()
    }

    #[wasm_bindgen(js_name = "prevHash")]
    pub fn prev_hash(&self) -> RpoDigest {
        self.0.prev_hash().into()
    }

    #[wasm_bindgen(js_name = "blockNum")]
    pub fn block_num(&self) -> u32 {
        self.0.block_num().as_u32()
    }

    #[wasm_bindgen(js_name = "chainRoot")]
    pub fn chain_root(&self) -> RpoDigest {
        self.0.chain_root().into()
    }

    #[wasm_bindgen(js_name = "accountRoot")]
    pub fn account_root(&self) -> RpoDigest {
        self.0.account_root().into()
    }

    #[wasm_bindgen(js_name = "nullifierRoot")]
    pub fn nullifier_root(&self) -> RpoDigest {
        self.0.nullifier_root().into()
    }

    #[wasm_bindgen(js_name = "noteRoot")]
    pub fn note_root(&self) -> RpoDigest {
        self.0.note_root().into()
    }

    #[wasm_bindgen(js_name = "txHash")]
    pub fn tx_hash(&self) -> RpoDigest {
        self.0.tx_hash().into()
    }

    #[wasm_bindgen(js_name = "kernelRoot")]
    pub fn kernel_root(&self) -> RpoDigest {
        self.0.kernel_root().into()
    }

    #[wasm_bindgen(js_name = "proofHash")]
    pub fn proof_hash(&self) -> RpoDigest {
        self.0.proof_hash().into()
    }

    pub fn timestamp(&self) -> u32 {
        self.0.timestamp()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeBlockHeader> for BlockHeader {
    fn from(header: NativeBlockHeader) -> Self {
        BlockHeader(header)
    }
}

impl From<&NativeBlockHeader> for BlockHeader {
    fn from(header: &NativeBlockHeader) -> Self {
        BlockHeader(header.clone())
    }
}

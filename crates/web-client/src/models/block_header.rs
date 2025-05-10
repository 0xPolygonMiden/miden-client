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

    pub fn commitment(&self) -> RpoDigest {
        self.0.commitment().into()
    }

    #[wasm_bindgen(js_name = "subCommitment")]
    pub fn sub_commitment(&self) -> RpoDigest {
        self.0.sub_commitment().into()
    }

    #[wasm_bindgen(js_name = "prevBlockCommitment")]
    pub fn prev_block_commitment(&self) -> RpoDigest {
        self.0.prev_block_commitment().into()
    }

    #[wasm_bindgen(js_name = "blockNum")]
    pub fn block_num(&self) -> u32 {
        self.0.block_num().as_u32()
    }

    #[wasm_bindgen(js_name = "chainCommitment")]
    pub fn chain_commitment(&self) -> RpoDigest {
        self.0.chain_commitment().into()
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

    #[wasm_bindgen(js_name = "txCommitment")]
    pub fn tx_commitment(&self) -> RpoDigest {
        self.0.tx_commitment().into()
    }

    #[wasm_bindgen(js_name = "txKernelCommitment")]
    pub fn tx_kernel_commitment(&self) -> RpoDigest {
        self.0.tx_kernel_commitment().into()
    }

    #[wasm_bindgen(js_name = "proofCommitment")]
    pub fn proof_commitment(&self) -> RpoDigest {
        self.0.proof_commitment().into()
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

impl From<BlockHeader> for NativeBlockHeader {
    fn from(header: BlockHeader) -> Self {
        header.0
    }
}

impl From<&BlockHeader> for NativeBlockHeader {
    fn from(header: &BlockHeader) -> Self {
        header.0.clone()
    }
}

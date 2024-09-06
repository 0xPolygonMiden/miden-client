use alloc::{string::String, vec::Vec};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BlockHeaderIdxdbObject {
    pub block_num: String,
    pub header: Vec<u8>,
    pub chain_mmr: String,
    pub has_client_notes: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ChainMmrNodeIdxdbObject {
    pub id: String,
    pub node: String,
}

#[derive(Serialize, Deserialize)]
pub struct MmrPeaksIdxdbObject {
    pub peaks: Option<Vec<u8>>,
}

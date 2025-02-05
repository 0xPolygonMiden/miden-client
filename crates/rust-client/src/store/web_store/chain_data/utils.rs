use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::num::NonZeroUsize;

use miden_objects::{block::BlockHeader, crypto::merkle::InOrderIndex, Digest};
use miden_tx::utils::Serializable;
use serde_wasm_bindgen::from_value;
use wasm_bindgen::JsValue;

use crate::store::{web_store::chain_data::ChainMmrNodeIdxdbObject, StoreError};

pub struct SerializedBlockHeaderData {
    pub block_num: String,
    pub header: Vec<u8>,
    pub chain_mmr_peaks: Vec<u8>,
    pub has_client_notes: bool,
}

pub struct SerializedChainMmrNodeData {
    pub id: String,
    pub node: String,
}

pub fn serialize_block_header(
    block_header: &BlockHeader,
    chain_mmr_peaks: &[Digest],
    has_client_notes: bool,
) -> Result<SerializedBlockHeaderData, StoreError> {
    let block_num = block_header.block_num().to_string();
    let header = block_header.to_bytes();
    let chain_mmr_peaks = chain_mmr_peaks.to_bytes();

    Ok(SerializedBlockHeaderData {
        block_num,
        header,
        chain_mmr_peaks,
        has_client_notes,
    })
}

pub fn serialize_chain_mmr_node(
    id: InOrderIndex,
    node: Digest,
) -> Result<SerializedChainMmrNodeData, StoreError> {
    let id: u64 = id.into();
    let id_as_str = id.to_string();
    let node = node.to_string();
    Ok(SerializedChainMmrNodeData { id: id_as_str, node })
}

#[allow(clippy::cast_possible_truncation)]
pub fn process_chain_mmr_nodes_from_js_value(
    js_value: JsValue,
) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
    let chain_mmr_nodes_idxdb: Vec<ChainMmrNodeIdxdbObject> = from_value(js_value).unwrap();

    let results: Result<BTreeMap<InOrderIndex, Digest>, StoreError> = chain_mmr_nodes_idxdb
        .into_iter()
        .map(|record| {
            let id_as_u64: u64 = record.id.parse::<u64>().unwrap();
            let id = InOrderIndex::new(NonZeroUsize::new(id_as_u64 as usize).unwrap());
            let node = Digest::try_from(&record.node)?;
            Ok((id, node))
        })
        .collect();

    results
}

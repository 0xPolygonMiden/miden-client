use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::num::NonZeroUsize;

use miden_objects::{crypto::merkle::InOrderIndex, BlockHeader, Digest};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::JsValue;

use crate::store::{web_store::chain_data::ChainMmrNodeIdxdbObject, StoreError};

pub struct SerializedBlockHeaderData {
    pub block_num: String,
    pub header: String,
    pub chain_mmr_peaks: String,
    pub has_client_notes: bool,
}

pub struct SerializedChainMmrNodeData {
    pub id: String,
    pub node: String,
}

pub fn serialize_block_header(
    block_header: BlockHeader,
    chain_mmr_peaks: Vec<Digest>,
    has_client_notes: bool,
) -> Result<SerializedBlockHeaderData, StoreError> {
    let block_num = block_header.block_num().to_string();
    let header =
        serde_json::to_string(&block_header).map_err(StoreError::InputSerializationError)?;
    let chain_mmr_peaks =
        serde_json::to_string(&chain_mmr_peaks).map_err(StoreError::InputSerializationError)?;

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
    let node = serde_json::to_string(&node).map_err(StoreError::InputSerializationError)?;
    Ok(SerializedChainMmrNodeData { id: id_as_str, node })
}

pub fn process_chain_mmr_nodes_from_js_value(
    js_value: JsValue,
) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
    let chain_mmr_nodes_idxdb: Vec<ChainMmrNodeIdxdbObject> = from_value(js_value).unwrap();

    let results: Result<BTreeMap<InOrderIndex, Digest>, StoreError> = chain_mmr_nodes_idxdb
        .into_iter()
        .map(|record| {
            let id_as_u64: u64 = record.id.parse::<u64>().unwrap();
            let id = InOrderIndex::new(NonZeroUsize::new(id_as_u64 as usize).unwrap());
            let node: Digest = serde_json::from_str(&record.node)
                .map_err(StoreError::JsonDataDeserializationError)?;
            Ok((id, node))
        })
        .collect();

    results
}

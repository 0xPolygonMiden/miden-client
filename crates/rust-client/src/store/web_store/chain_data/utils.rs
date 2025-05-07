use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::num::NonZeroUsize;

use miden_objects::{Digest, block::BlockHeader, crypto::merkle::InOrderIndex};
use miden_tx::utils::Serializable;
use serde_wasm_bindgen::from_value;
use wasm_bindgen::JsValue;

use crate::store::{StoreError, web_store::chain_data::PartialBlockchainNodeIdxdbObject};

pub struct SerializedBlockHeaderData {
    pub block_num: String,
    pub header: Vec<u8>,
    pub partial_blockchain_peaks: Vec<u8>,
    pub has_client_notes: bool,
}

pub struct SerializedPartialBlockchainNodeData {
    pub id: String,
    pub node: String,
}

pub fn serialize_block_header(
    block_header: &BlockHeader,
    partial_blockchain_peaks: &[Digest],
    has_client_notes: bool,
) -> Result<SerializedBlockHeaderData, StoreError> {
    let block_num = block_header.block_num().to_string();
    let header = block_header.to_bytes();
    let partial_blockchain_peaks = partial_blockchain_peaks.to_bytes();

    Ok(SerializedBlockHeaderData {
        block_num,
        header,
        partial_blockchain_peaks,
        has_client_notes,
    })
}

pub fn serialize_partial_blockchain_node(
    id: InOrderIndex,
    node: Digest,
) -> Result<SerializedPartialBlockchainNodeData, StoreError> {
    let id: u64 = id.into();
    let id_as_str = id.to_string();
    let node = node.to_string();
    Ok(SerializedPartialBlockchainNodeData { id: id_as_str, node })
}

pub fn process_partial_blockchain_nodes_from_js_value(
    js_value: JsValue,
) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
    let partial_blockchain_nodes_idxdb: Vec<PartialBlockchainNodeIdxdbObject> =
        from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

    let results: Result<BTreeMap<InOrderIndex, Digest>, StoreError> =
        partial_blockchain_nodes_idxdb
            .into_iter()
            .map(|record| {
                let id_as_u64: u64 = record.id.parse::<u64>().unwrap();
                let id = InOrderIndex::new(
                    NonZeroUsize::new(
                        usize::try_from(id_as_u64)
                            .expect("usize should not fail converting to u64"),
                    )
                    .unwrap(),
                );
                let node = Digest::try_from(&record.node)?;
                Ok((id, node))
            })
            .collect();

    results
}

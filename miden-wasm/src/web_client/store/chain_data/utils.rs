use miden_client::errors::StoreError;
use miden_objects::{crypto::merkle::InOrderIndex, BlockHeader, Digest};
// use crate::native_code::errors::StoreError;

type SerializedBlockHeaderData = (String, String, String, bool);
type SerializedChainMmrNodeData = (String, String);

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

    Ok((block_num, header, chain_mmr_peaks, has_client_notes))
}

pub fn serialize_chain_mmr_node(
    id: InOrderIndex,
    node: Digest,
) -> Result<SerializedChainMmrNodeData, StoreError> {
    let id: u64 = id.into();
    let id_as_str = id.to_string();
    let node = serde_json::to_string(&node).map_err(StoreError::InputSerializationError)?;
    Ok((id_as_str, node))
}

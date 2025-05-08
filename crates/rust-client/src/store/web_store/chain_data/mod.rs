use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    Digest,
    block::{BlockHeader, BlockNumber},
    crypto::merkle::{InOrderIndex, MmrPeaks},
};
use miden_tx::utils::Deserializable;
use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::JsFuture;

use super::WebStore;
use crate::store::{PartialBlockchainFilter, StoreError};

mod js_bindings;
use js_bindings::{
    idxdb_get_block_headers, idxdb_get_partial_blockchain_nodes,
    idxdb_get_partial_blockchain_nodes_all, idxdb_get_partial_blockchain_peaks_by_block_num,
    idxdb_get_tracked_block_headers, idxdb_insert_block_header,
    idxdb_insert_partial_blockchain_nodes, idxdb_prune_irrelevant_blocks,
};

mod models;
use models::{
    BlockHeaderIdxdbObject, PartialBlockchainNodeIdxdbObject, PartialBlockchainPeaksIdxdbObject,
};

pub mod utils;
use utils::{
    SerializedBlockHeaderData, SerializedPartialBlockchainNodeData,
    process_partial_blockchain_nodes_from_js_value, serialize_block_header,
    serialize_partial_blockchain_node,
};

impl WebStore {
    pub(crate) async fn insert_block_header(
        &self,
        block_header: &BlockHeader,
        partial_blockchain_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        let partial_blockchain_peaks = partial_blockchain_peaks.peaks().to_vec();
        let SerializedBlockHeaderData {
            block_num,
            header,
            partial_blockchain_peaks,
            has_client_notes,
        } = serialize_block_header(block_header, &partial_blockchain_peaks, has_client_notes)?;

        let promise = idxdb_insert_block_header(
            block_num,
            header,
            partial_blockchain_peaks,
            has_client_notes,
        );
        JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to insert block header: {js_error:?}",))
        })?;

        Ok(())
    }

    pub(crate) async fn get_block_headers(
        &self,
        block_numbers: &BTreeSet<BlockNumber>,
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError> {
        let formatted_block_numbers_list: Vec<String> = block_numbers
            .iter()
            .map(|block_number| i64::from(block_number.as_u32()).to_string())
            .collect();

        let promise = idxdb_get_block_headers(formatted_block_numbers_list);
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to get block headers: {js_error:?}",))
        })?;
        let block_headers_idxdb: Vec<Option<BlockHeaderIdxdbObject>> = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        // Transform the list of Option<BlockHeaderIdxdbObject> to a list of results
        let results: Result<Vec<(BlockHeader, bool)>, StoreError> = block_headers_idxdb
            .into_iter()
            .filter_map(|record_option| record_option.map(Ok))
            .map(|record_result: Result<BlockHeaderIdxdbObject, StoreError>| {
                let record = record_result?;
                let block_header = BlockHeader::read_from_bytes(&record.header)?;
                let has_client_notes = record.has_client_notes;

                Ok((block_header, has_client_notes))
            })
            .collect(); // Collects into Result<Vec<(BlockHeader, bool)>, StoreError>

        results
    }

    pub(crate) async fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError> {
        let promise = idxdb_get_tracked_block_headers();
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to get tracked block headers: {js_error:?}",))
        })?;
        let block_headers_idxdb: Vec<BlockHeaderIdxdbObject> = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        let results: Result<Vec<BlockHeader>, StoreError> = block_headers_idxdb
            .into_iter()
            .map(|record| {
                let block_header = BlockHeader::read_from_bytes(&record.header)?;

                Ok(block_header)
            })
            .collect();

        results
    }

    pub(crate) async fn get_partial_blockchain_nodes(
        &self,
        filter: PartialBlockchainFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        match filter {
            PartialBlockchainFilter::All => {
                let promise = idxdb_get_partial_blockchain_nodes_all();
                let js_value = JsFuture::from(promise).await.map_err(|js_error| {
                    StoreError::DatabaseError(format!(
                        "failed to get all partial blockchain nodes: {js_error:?}",
                    ))
                })?;
                process_partial_blockchain_nodes_from_js_value(js_value)
            },
            PartialBlockchainFilter::List(ids) => {
                let formatted_list: Vec<String> =
                    ids.iter().map(|id| (Into::<u64>::into(*id)).to_string()).collect();

                let promise = idxdb_get_partial_blockchain_nodes(formatted_list);
                let js_value = JsFuture::from(promise).await.map_err(|js_error| {
                    StoreError::DatabaseError(format!(
                        "failed to get partial blockchain nodes: {js_error:?}",
                    ))
                })?;
                process_partial_blockchain_nodes_from_js_value(js_value)
            },
        }
    }

    pub(crate) async fn get_partial_blockchain_peaks_by_block_num(
        &self,
        block_num: BlockNumber,
    ) -> Result<MmrPeaks, StoreError> {
        let block_num_as_str = block_num.to_string();

        let promise = idxdb_get_partial_blockchain_peaks_by_block_num(block_num_as_str);
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!(
                "failed to get partial blockchain peaks by block number: {js_error:?}",
            ))
        })?;
        let mmr_peaks_idxdb: PartialBlockchainPeaksIdxdbObject = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        if let Some(peaks) = mmr_peaks_idxdb.peaks {
            let mmr_peaks_nodes: Vec<Digest> = Vec::<Digest>::read_from_bytes(&peaks)?;

            return MmrPeaks::new(block_num.as_usize(), mmr_peaks_nodes)
                .map_err(StoreError::MmrError);
        }

        Ok(MmrPeaks::new(0, vec![])?)
    }

    pub(crate) async fn insert_partial_blockchain_nodes(
        &self,
        nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError> {
        let mut serialized_node_ids = Vec::new();
        let mut serialized_nodes = Vec::new();
        for (id, node) in nodes {
            let SerializedPartialBlockchainNodeData { id, node } =
                serialize_partial_blockchain_node(*id, *node)?;
            serialized_node_ids.push(id);
            serialized_nodes.push(node);
        }

        let promise = idxdb_insert_partial_blockchain_nodes(serialized_node_ids, serialized_nodes);
        JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!(
                "failed to insert partial blockchain nodes: {js_error:?}",
            ))
        })?;

        Ok(())
    }

    /// This function isn't used in this crate, rather it is used in the 'miden-client' crate.
    /// The reference is [found here](https://github.com/0xPolygonMiden/miden-client/blob/c273847726ed325d2e627e4db18bf9f3ab8c28ba/src/store/sqlite_store/sync.rs#L105)
    /// It is duplicated here due to its reliance on the store.
    #[allow(dead_code)]
    pub(crate) async fn insert_block_header_tx(
        block_header: &BlockHeader,
        partial_blockchain_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        let partial_blockchain_peaks = partial_blockchain_peaks.peaks().to_vec();
        let SerializedBlockHeaderData {
            block_num,
            header,
            partial_blockchain_peaks,
            has_client_notes,
        } = serialize_block_header(block_header, &partial_blockchain_peaks, has_client_notes)?;

        let promise = idxdb_insert_block_header(
            block_num,
            header,
            partial_blockchain_peaks,
            has_client_notes,
        );
        JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to insert block header: {js_error:?}",))
        })?;

        Ok(())
    }

    pub(crate) async fn prune_irrelevant_blocks(&self) -> Result<(), StoreError> {
        let promise = idxdb_prune_irrelevant_blocks();
        JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to prune block header: {js_error:?}",))
        })?;

        Ok(())
    }
}

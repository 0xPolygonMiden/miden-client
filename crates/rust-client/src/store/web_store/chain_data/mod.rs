use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    crypto::merkle::{InOrderIndex, MmrPeaks},
    BlockHeader, Digest,
};
use miden_tx::utils::Deserializable;
use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::JsFuture;

use super::WebStore;
use crate::store::{ChainMmrNodeFilter, StoreError};

mod js_bindings;
use js_bindings::*;

mod models;
use models::*;

pub mod utils;
use utils::*;

impl WebStore {
    pub(crate) async fn insert_block_header(
        &self,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        let chain_mmr_peaks = chain_mmr_peaks.peaks().to_vec();
        let serialized_data =
            serialize_block_header(block_header, chain_mmr_peaks, has_client_notes)?;

        let promise = idxdb_insert_block_header(
            serialized_data.block_num,
            serialized_data.header,
            serialized_data.chain_mmr_peaks,
            serialized_data.has_client_notes,
        );
        JsFuture::from(promise).await.unwrap();

        Ok(())
    }

    pub(crate) async fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError> {
        let formatted_block_numbers_list: Vec<String> = block_numbers
            .iter()
            .map(|block_number| (*block_number as i64).to_string())
            .collect();

        let promise = idxdb_get_block_headers(formatted_block_numbers_list);
        let js_value = JsFuture::from(promise).await.unwrap();
        let block_headers_idxdb: Vec<Option<BlockHeaderIdxdbObject>> =
            from_value(js_value).unwrap();

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
        let js_value = JsFuture::from(promise).await.unwrap();
        let block_headers_idxdb: Vec<BlockHeaderIdxdbObject> = from_value(js_value).unwrap();

        let results: Result<Vec<BlockHeader>, StoreError> = block_headers_idxdb
            .into_iter()
            .map(|record| {
                let block_header = BlockHeader::read_from_bytes(&record.header)?;

                Ok(block_header)
            })
            .collect();

        results
    }

    pub(crate) async fn get_chain_mmr_nodes<'a>(
        &'a self,
        filter: ChainMmrNodeFilter<'a>,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        match filter {
            ChainMmrNodeFilter::All => {
                let promise = idxdb_get_chain_mmr_nodes_all();
                let js_value = JsFuture::from(promise).await.unwrap();
                process_chain_mmr_nodes_from_js_value(js_value)
            },
            ChainMmrNodeFilter::List(ids) => {
                let formatted_list: Vec<String> =
                    ids.iter().map(|id| (Into::<u64>::into(*id)).to_string()).collect();

                let promise = idxdb_get_chain_mmr_nodes(formatted_list);
                let js_value = JsFuture::from(promise).await.unwrap();
                process_chain_mmr_nodes_from_js_value(js_value)
            },
        }
    }

    pub(crate) async fn get_chain_mmr_peaks_by_block_num(
        &self,
        block_num: u32,
    ) -> Result<MmrPeaks, StoreError> {
        let block_num_as_str = block_num.to_string();

        let promise = idxdb_get_chain_mmr_peaks_by_block_num(block_num_as_str);
        let js_value = JsFuture::from(promise).await.unwrap();
        let mmr_peaks_idxdb: MmrPeaksIdxdbObject = from_value(js_value).unwrap();

        if let Some(peaks) = mmr_peaks_idxdb.peaks {
            let mmr_peaks_nodes: Vec<Digest> = Vec::<Digest>::read_from_bytes(&peaks)?;

            return MmrPeaks::new(block_num as usize, mmr_peaks_nodes)
                .map_err(StoreError::MmrError);
        }

        Ok(MmrPeaks::new(0, vec![])?)
    }

    pub(crate) async fn insert_chain_mmr_nodes(
        &self,
        nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError> {
        let mut serialized_node_ids = Vec::new();
        let mut serialized_nodes = Vec::new();
        for (id, node) in nodes.iter() {
            let serialized_data = serialize_chain_mmr_node(*id, *node)?;
            serialized_node_ids.push(serialized_data.id);
            serialized_nodes.push(serialized_data.node);
        }

        let promise = idxdb_insert_chain_mmr_nodes(serialized_node_ids, serialized_nodes);
        JsFuture::from(promise).await.unwrap();

        Ok(())
    }

    /// This function is not used in this crate, rather it is used in the 'miden-client' crate.
    /// https://github.com/0xPolygonMiden/miden-client/blob/c273847726ed325d2e627e4db18bf9f3ab8c28ba/src/store/sqlite_store/sync.rs#L105
    /// It is duplicated here due to its reliance on the store.
    #[allow(dead_code)]
    pub(crate) async fn insert_block_header_tx(
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        let chain_mmr_peaks = chain_mmr_peaks.peaks().to_vec();
        let serialized_data =
            serialize_block_header(block_header, chain_mmr_peaks, has_client_notes)?;

        let promise = idxdb_insert_block_header(
            serialized_data.block_num,
            serialized_data.header,
            serialized_data.chain_mmr_peaks,
            serialized_data.has_client_notes,
        );
        JsFuture::from(promise).await.unwrap();

        Ok(())
    }
}

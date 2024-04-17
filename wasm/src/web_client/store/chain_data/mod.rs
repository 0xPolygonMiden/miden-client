// use wasm_bindgen_futures::JsFuture;
// use serde_wasm_bindgen::from_value;

// use crate::native_code::store::ChainMmrNodeFilter;

use super::WebStore;

// mod js_bindings;
// use js_bindings::*;

// mod models;
// use models::*;

// mod utils;
// use utils::*;

impl WebStore {
    // pub(crate) async fn insert_block_header(
    //     &self,
    //     block_header: BlockHeader,
    //     chain_mmr_peaks: MmrPeaks,
    //     has_client_notes: bool
    // ) -> Result<(), ()> {
    //     let chain_mmr_peaks = chain_mmr_peaks.peaks().to_vec();
    //     let (block_num, header, chain_mmr, has_client_notes) =
    //         serialize_block_header(block_header, chain_mmr_peaks, has_client_notes)?;

    //     let promise = idxdb_insert_block_header(
    //         block_num,
    //         header,
    //         chain_mmr,
    //         has_client_notes
    //     );
    //     let js_value = JsFuture::from(promise).await;

    //     Ok(())
    // }

    // pub(crate) async fn get_block_headers(
    //     &self,
    //     block_numbers: &[u32],
    // ) -> Result<Vec<(BlockHeader, bool)>, ()> {
    //     let formatted_block_numbers_list: Vec<String> = block_numbers
    //         .iter()
    //         .map(|block_number| (*block_number as i64).to_string())
    //         .collect();

    //     let promise = idxdb_get_block_headers(formatted_block_numbers_list);
    //     let js_value = JsFuture::from(promise).await?;
    //     let block_headers_idxdb: Vec<BlockHeaderIdxdbObject> = from_value(js_value).unwrap();

    //     let results:Result<Vec<(BlockHeader, bool)>, ()> = block_headers_idxdb.into_iter().map(|record| {
    //         let block_header = serde_json::from_str(&record.header).unwrap();
    //         let has_client_notes = record.has_client_notes;

    //         Ok((block_header, has_client_notes))
    //     }).collect();

    //     return results;
    // }

    // pub(crate) async fn get_tracked_block_headers(
    //     &self
    // ) -> Result<Vec<BlockHeader>, ()> {
    //     let promise = idxdb_get_tracked_block_headers();
    //     let js_value = JsFuture::from(promise).await?;
    //     let block_headers_idxdb: Vec<BlockHeaderIdxdbObject> = from_value(js_value).unwrap();

    //     let results:Result<Vec<(BlockHeader, bool)>, ()> = block_headers_idxdb.into_iter().map(|record| {
    //         let block_header = serde_json::from_str(&record.header).unwrap();
    //         let has_client_notes = record.has_client_notes;

    //         Ok((block_header, has_client_notes))
    //     }).collect();

    //     return results;
    // }

    // pub(crate) async fn get_chain_mmr_nodes(
    //     &self,
    //     filter: ChainMmrNodeFilter,
    // ) -> Result<BTreeMap<InOrderIndex, Digest>, ()> {
    //     match filter {
    //         ChainMmrNodeFilter::All => {
    //             let promise = idxdb_get_chain_mmr_nodes_all();
    //             let js_value = JsFuture::from(promise).await?;
    //             let chain_mmr_nodes_idxdb: Vec<ChainMmrNodeIdxdbObject> = from_value(js_value).unwrap();

    //             let results:Result<BTreeMap<InOrderIndex, Digest>, ()> = chain_mmr_nodes_idxdb.into_iter().map(|record| {
    //                 let id_as_u64: u64 = record.id.parse::<u64>().unwrap();
    //                 let id = InOrderIndex::new(NonZeroUsize::new(id_as_u64 as usize).unwrap());
    //                 let node: Digest =
    //                     serde_json::from_str(&record.node).map_err(|err| ())?;
    //                 Ok((id, node))
    //             }).collect();

    //             return results;
    //         },
    //         ChainMmrNodeFilter::List(ids) => {
    //             let formatted_list: Vec<String> = ids
    //                 .iter()
    //                 .map(|id| (Into::<u64>::into(*id)).to_string())
    //                 .collect();

    //             let promise = idxdb_get_chain_mmr_nodes(ids);
    //             let js_value = JsFuture::from(promise).await?;
    //             let chain_mmr_nodes_idxdb: Vec<ChainMmrNodeIdxdbObject> = from_value(js_value).unwrap();

    //             let results:Result<BTreeMap<InOrderIndex, Digest>, ()> = chain_mmr_nodes_idxdb.into_iter().map(|record| {
    //                 let id_as_u64: u64 = record.id.parse::<u64>().unwrap();
    //                 let id = InOrderIndex::new(NonZeroUsize::new(id_as_u64 as usize).unwrap());
    //                 let node: Digest =
    //                     serde_json::from_str(&record.node).map_err(|err| ())?;
    //                 Ok((id, node))
    //             }).collect();

    //             return results;
    //         }
    //     }
    // }

    // pub(crate) async fn get_chain_mmr_peaks_by_block_num(
    //     &self,
    //     block_num: u32,
    // ) -> Result<MmrPeaks, ()> {
    //     let block_num_as_str = block_num.to_string();
        
    //     let promise = idxdb_get_chain_mmr_peaks_by_block_num(block_num_as_str);
    //     let js_value = JsFuture::from(promise).await?;
    //     let mmr_peaks_idxdb: MmrPeaksIdxdbObject = from_value(js_value).unwrap();

    //     if let Some(peaks) = mmr_peaks_idxdb.peaks {
    //         let mmr_peaks_nodes: Vec<Digest> =
    //             serde_json::from_str(&peaks).map_err(|err| ())?;

    //         return MmrPeaks::new(block_num as usize, mmr_peaks_nodes).map_err(|err| ())
    //     }

    //     return Ok(MmrPeaks::new(0, vec![])?);
    // }
}
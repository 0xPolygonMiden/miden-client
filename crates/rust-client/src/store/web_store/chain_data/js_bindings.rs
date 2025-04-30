use alloc::{string::String, vec::Vec};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{js_sys, wasm_bindgen};

// ChainData IndexedDB Operations
#[wasm_bindgen(module = "/src/store/web_store/js/chainData.js")]
extern "C" {
    // GETS
    // ================================================================================================

    #[wasm_bindgen(js_name = getBlockHeaders)]
    pub fn idxdb_get_block_headers(block_numbers: Vec<String>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getTrackedBlockHeaders)]
    pub fn idxdb_get_tracked_block_headers() -> js_sys::Promise;

    #[wasm_bindgen(js_name = getChainMmrNodesAll)]
    pub fn idxdb_get_chain_mmr_nodes_all() -> js_sys::Promise;

    #[wasm_bindgen(js_name = getChainMmrNodes)]
    pub fn idxdb_get_chain_mmr_nodes(ids: Vec<String>) -> js_sys::Promise;

    #[wasm_bindgen(js_name = getChainMmrPeaksByBlockNum)]
    pub fn idxdb_get_chain_mmr_peaks_by_block_num(block_num: String) -> js_sys::Promise;

    // INSERTS
    // ================================================================================================

    #[wasm_bindgen(js_name = insertBlockHeader)]
    pub fn idxdb_insert_block_header(
        block_num: String,
        header: Vec<u8>,
        chain_mmr_peaks: Vec<u8>,
        has_client_notes: bool,
    ) -> js_sys::Promise;

    #[wasm_bindgen(js_name = insertChainMmrNodes)]
    pub fn idxdb_insert_chain_mmr_nodes(ids: Vec<String>, nodes: Vec<String>) -> js_sys::Promise;

    // DELETES
    // ================================================================================================

    #[wasm_bindgen(js_name = pruneIrrelevantBlocks)]
    pub fn idxdb_prune_irrelevant_blocks() -> js_sys::Promise;
}

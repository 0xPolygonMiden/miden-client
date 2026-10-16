//! Repairs a store that tracks a block without its full MMR path.
//!
//! A store gets into that state when a writer stores a path for an older forest than the forest of
//! the stored sync height. The store then cannot build its partial MMR, so every sync and import
//! fails. The nodes that are missing are part of the block proof that the node returns.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::iter;

use miden_protocol::Word;
use miden_protocol::block::{BlockHeader, BlockNumber};
use miden_protocol::crypto::merkle::mmr::{InOrderIndex, MmrProof, PartialMmr};
use tracing::warn;

use super::block_header::fetch_block_header;
use crate::ClientError;
use crate::rpc::NodeRpcClient;
use crate::store::{PartialBlockchainFilter, Store, StoreError, tracked_leaf_path_indices};

/// The nodes to store for one tracked block.
struct Repair {
    block_header: BlockHeader,
    missing_nodes: Vec<(InOrderIndex, Word)>,
}

/// Why the heal stops before it stores every missing node.
enum Stop {
    /// Another writer can have changed the store after the failed read, so a new read can succeed.
    StoreChanged(String),
    /// The node or its proof cannot repair the store. The heal writes nothing.
    Unrepairable(String),
    /// A store call failed.
    Store(StoreError),
}

impl From<StoreError> for Stop {
    fn from(err: StoreError) -> Self {
        Self::Store(err)
    }
}

/// Stores the nodes that tracked blocks need, then reads the partial MMR of `store` once and
/// returns the result of that read.
///
/// `error` is the error that the store returned when it built the partial MMR. For each tracked
/// block that does not have its full path, the heal requests the block proof from the node. It
/// checks every proof against the stored header, the stored peaks and the stored nodes before it
/// writes, and it stores only the nodes that are missing. It never marks a block as tracked.
///
/// If another writer changed the store after the failed read, for example if it repaired the store,
/// moved the sync height, untracked a block or caused the store to refuse a write, the heal stops
/// and returns the result of a new read. A later read repairs the store again if it is still
/// broken. If the node is not reachable, if its proof is for a smaller forest or for a different
/// chain, or if a stored node is different from the proof, the heal writes nothing and returns
/// `error`.
pub(crate) async fn heal(
    store: &Arc<dyn Store>,
    rpc_api: &Arc<dyn NodeRpcClient>,
    error: StoreError,
) -> Result<PartialMmr, ClientError> {
    let mut repaired_blocks = BTreeSet::new();
    match repair_blocks(store, rpc_api, &mut repaired_blocks).await {
        Ok(()) => {
            let result = store.get_current_partial_mmr().await;
            match &result {
                Ok(_) => warn!(
                    blocks = ?repaired_blocks,
                    "stored the missing MMR nodes of tracked blocks"
                ),
                Err(err) => warn!(
                    blocks = ?repaired_blocks,
                    %err,
                    "the partial MMR is not readable after the repair of tracked blocks"
                ),
            }
            result.map_err(Into::into)
        },
        Err(Stop::StoreChanged(reason)) => {
            warn!(
                blocks = ?repaired_blocks,
                %reason,
                "the store changed during the repair of tracked blocks; reading the MMR again"
            );
            store.get_current_partial_mmr().await.map_err(Into::into)
        },
        Err(Stop::Unrepairable(reason)) => {
            warn!(
                blocks = ?repaired_blocks,
                %reason,
                "cannot store the missing MMR nodes of tracked blocks"
            );
            Err(error.into())
        },
        Err(Stop::Store(err)) => {
            warn!(
                blocks = ?repaired_blocks,
                %err,
                "cannot store the missing MMR nodes of tracked blocks"
            );
            Err(err.into())
        },
    }
}

/// Plans the repair of every broken tracked block, then stores the missing nodes. Adds each block
/// that it repairs to `repaired_blocks`.
async fn repair_blocks(
    store: &Arc<dyn Store>,
    rpc_api: &Arc<dyn NodeRpcClient>,
    repaired_blocks: &mut BTreeSet<BlockNumber>,
) -> Result<(), Stop> {
    for repair in plan_repairs(store, rpc_api).await? {
        let block_num = repair.block_header.block_num();
        // `false` never marks the block as tracked. If another writer untracked the block, it stays
        // untracked.
        match store
            .insert_block_header(&repair.block_header, &repair.missing_nodes, false)
            .await
        {
            Ok(()) => {
                repaired_blocks.insert(block_num);
            },
            Err(StoreError::DatabaseTransientError(reason)) => {
                return Err(Stop::StoreChanged(reason));
            },
            Err(err) => return Err(err.into()),
        }
    }
    Ok(())
}

/// Finds the tracked blocks that do not have their full path, and returns the nodes to store for
/// each of them. Each block needs one proof request.
async fn plan_repairs(
    store: &Arc<dyn Store>,
    rpc_api: &Arc<dyn NodeRpcClient>,
) -> Result<Vec<Repair>, Stop> {
    let peaks = store.get_current_blockchain_peaks().await?;
    let sync_height = block_number(peaks.num_leaves())?;
    // A sync of another writer can prune the header of the sync height that the peaks are for.
    let Some((sync_header, _)) = store.get_block_header_by_num(sync_height).await? else {
        return Err(Stop::StoreChanged(format!(
            "the header of the sync height {sync_height} is not stored"
        )));
    };

    // Each path is checked against this partial MMR, which has only the stored peaks.
    let mut peaks_mmr = PartialMmr::from_peaks(peaks);
    peaks_mmr.add(sync_header.commitment(), false).map_err(StoreError::MmrError)?;
    let forest = peaks_mmr.forest();

    let tracked_blocks = store.get_tracked_block_header_numbers().await?;
    let nodes = store
        .get_partial_blockchain_nodes(PartialBlockchainFilter::Forest(forest))
        .await?;

    let mut broken_blocks = Vec::new();
    for block in tracked_blocks {
        let Some(indices) = tracked_leaf_path_indices(sync_height, block) else {
            return Err(Stop::StoreChanged(format!(
                "tracked block {block} is after the sync height {sync_height}"
            )));
        };
        if indices.iter().any(|index| !nodes.contains_key(index)) {
            broken_blocks.push((block_number(block)?, indices));
        }
    }
    if broken_blocks.is_empty() {
        return Err(Stop::StoreChanged("every tracked block has its full path".to_string()));
    }

    let mut repairs = Vec::with_capacity(broken_blocks.len());
    for (block_num, indices) in broken_blocks {
        let (block_header, mmr_proof) =
            fetch_block_header(rpc_api, block_num).await.map_err(|err| {
                Stop::Unrepairable(format!("cannot get the proof of block {block_num}: {err}"))
            })?;

        // Another writer can untrack the block while the request runs.
        let Some((stored_header, relevance)) = store.get_block_header_by_num(block_num).await?
        else {
            return Err(Stop::StoreChanged(format!(
                "the header of block {block_num} is not stored"
            )));
        };
        if !bool::from(relevance) {
            return Err(Stop::StoreChanged(format!("block {block_num} is not tracked")));
        }
        if stored_header != block_header {
            return Err(Stop::Unrepairable(format!(
                "the stored header of block {block_num} is not the header from the node"
            )));
        }

        let missing_nodes = missing_nodes(&peaks_mmr, &nodes, &block_header, &mmr_proof, indices)
            .map_err(Stop::Unrepairable)?;
        repairs.push(Repair { block_header, missing_nodes });
    }

    Ok(repairs)
}

/// Returns the nodes at `indices` that are not in `nodes`, with their values from `mmr_proof`.
///
/// `indices` are the indices of the leaf and the path of the block at the forest of `peaks_mmr`.
/// Returns the reason as an error if the proof is not for that forest, if it does not match the
/// peaks, or if a node in `nodes` is different from the proof.
fn missing_nodes(
    peaks_mmr: &PartialMmr,
    nodes: &BTreeMap<InOrderIndex, Word>,
    block_header: &BlockHeader,
    mmr_proof: &MmrProof,
    indices: Vec<InOrderIndex>,
) -> Result<Vec<(InOrderIndex, Word)>, String> {
    let block_num = block_header.block_num();
    let mmr_proof = mmr_proof.with_forest(peaks_mmr.forest()).map_err(|err| {
        format!("the proof of block {block_num} is not for the stored forest: {err}")
    })?;
    peaks_mmr
        .clone()
        .track(block_num.as_usize(), block_header.commitment(), mmr_proof.merkle_path())
        .map_err(|err| {
            format!("the proof of block {block_num} does not match the stored peaks: {err}")
        })?;

    // SQLite keeps a stored node on conflict, and IndexedDB refuses the insert. So a stored node
    // that is different from the proof stops the heal.
    let values = iter::once(block_header.commitment())
        .chain(mmr_proof.merkle_path().nodes().iter().copied());
    let mut missing_nodes = Vec::new();
    for (index, value) in indices.into_iter().zip(values) {
        match nodes.get(&index) {
            Some(stored) if *stored != value => {
                return Err(format!(
                    "the stored node {} of block {block_num} is not the node from the proof",
                    index.inner()
                ));
            },
            Some(_) => {},
            None => missing_nodes.push((index, value)),
        }
    }
    Ok(missing_nodes)
}

fn block_number(block: usize) -> Result<BlockNumber, StoreError> {
    u32::try_from(block)
        .map(BlockNumber::from)
        .map_err(|err| StoreError::ParsingError(err.to_string()))
}

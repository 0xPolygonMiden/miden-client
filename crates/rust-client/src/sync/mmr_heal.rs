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
use miden_protocol::crypto::merkle::mmr::{InOrderIndex, MmrError, MmrProof, PartialMmr};
use tracing::warn;

use super::block_header::fetch_block_header;
use crate::ClientError;
use crate::rpc::NodeRpcClient;
use crate::store::{PartialBlockchainFilter, Store, StoreError, tracked_leaf_path_indices};

/// The number of times the heal stores missing nodes and reads the partial MMR again.
const MAX_HEAL_ROUNDS: usize = 3;

/// The number of broken blocks that one round repairs.
const MAX_BLOCKS_PER_ROUND: usize = 64;

/// The nodes to store for one tracked block.
struct Repair {
    block_header: BlockHeader,
    missing_nodes: Vec<(InOrderIndex, Word)>,
}

/// What a heal round can do.
enum Plan {
    /// Store these nodes.
    Store(Vec<Repair>),
    /// The heal cannot repair the store now, for the given reason. The heal writes nothing.
    Stop(String),
}

/// Stores the nodes that tracked blocks need, then returns the partial MMR of `store`.
///
/// `error` is the error that the store returned when it built the partial MMR. For each tracked
/// block that does not have its full path, the heal requests the block proof from the node. It
/// checks the proof against the stored header, the stored peaks and the stored nodes, and it stores
/// only the nodes that are missing. It never marks a block as tracked.
///
/// If the node is not reachable, if its proof is for a smaller forest or for a different chain, or
/// if a stored node is different from the proof, the heal writes nothing and returns `error`.
pub(crate) async fn heal(
    store: &Arc<dyn Store>,
    rpc_api: &Arc<dyn NodeRpcClient>,
    error: StoreError,
) -> Result<PartialMmr, ClientError> {
    let mut proofs = BTreeMap::new();
    let mut repaired_blocks = BTreeSet::new();

    for round in 0..=MAX_HEAL_ROUNDS {
        match store.get_current_partial_mmr().await {
            Ok(partial_mmr) => {
                if !repaired_blocks.is_empty() {
                    warn!(blocks = ?repaired_blocks, "stored the missing MMR nodes of tracked blocks");
                }
                return Ok(partial_mmr);
            },
            Err(StoreError::MmrError(MmrError::InconsistentPartialMmr(_))) => {},
            Err(err) => return Err(err.into()),
        }
        if round == MAX_HEAL_ROUNDS {
            break;
        }

        let repairs = match plan_repairs(store, rpc_api, &mut proofs).await? {
            Plan::Store(repairs) => repairs,
            Plan::Stop(reason) => {
                warn!(%reason, "cannot store the missing MMR nodes of tracked blocks");
                return Err(error.into());
            },
        };

        for repair in repairs {
            let block_num = repair.block_header.block_num();
            // `false` never marks the block as tracked. If another writer untracked the block, it
            // stays untracked.
            match store
                .insert_block_header(&repair.block_header, &repair.missing_nodes, false)
                .await
            {
                Ok(()) => {
                    repaired_blocks.insert(block_num);
                },
                // Another writer changed the chain data. The next round reads it again.
                Err(StoreError::DatabaseTransientError(_)) => break,
                Err(err) => return Err(err.into()),
            }
        }
    }

    Err(error.into())
}

/// Finds the tracked blocks that do not have their full path, and returns the nodes to store for
/// each of them.
///
/// `proofs` keeps the block proofs from earlier rounds, so that each block needs one request.
async fn plan_repairs(
    store: &Arc<dyn Store>,
    rpc_api: &Arc<dyn NodeRpcClient>,
    proofs: &mut BTreeMap<BlockNumber, (BlockHeader, MmrProof)>,
) -> Result<Plan, StoreError> {
    let peaks = store.get_current_blockchain_peaks().await?;
    let sync_height = block_number(peaks.num_leaves())?;
    let (sync_header, _) = store
        .get_block_header_by_num(sync_height)
        .await?
        .ok_or(StoreError::BlockHeaderNotFound(sync_height))?;

    // Each path is checked against this partial MMR, which has only the stored peaks.
    let mut peaks_mmr = PartialMmr::from_peaks(peaks);
    peaks_mmr.add(sync_header.commitment(), false)?;
    let forest = peaks_mmr.forest();

    let tracked_blocks = store.get_tracked_block_header_numbers().await?;
    let nodes = store
        .get_partial_blockchain_nodes(PartialBlockchainFilter::Forest(forest))
        .await?;

    let mut broken_blocks = Vec::new();
    for block in tracked_blocks {
        let Some(indices) = tracked_leaf_path_indices(forest, block) else {
            return Ok(Plan::Stop(format!(
                "tracked block {block} is after the sync height {sync_height}"
            )));
        };
        if indices.iter().any(|index| !nodes.contains_key(index)) {
            broken_blocks.push((block_number(block)?, indices));
        }
    }
    if broken_blocks.is_empty() {
        return Ok(Plan::Stop("every tracked block has its full path".to_string()));
    }

    let mut repairs = Vec::new();
    for (block_num, indices) in broken_blocks.into_iter().take(MAX_BLOCKS_PER_ROUND) {
        let (block_header, mmr_proof) = match proofs.get(&block_num) {
            Some(fetched) => fetched.clone(),
            None => match fetch_block_header(rpc_api, block_num).await {
                Ok(fetched) => {
                    proofs.insert(block_num, fetched.clone());
                    fetched
                },
                Err(err) => {
                    return Ok(Plan::Stop(format!(
                        "cannot get the proof of block {block_num}: {err}"
                    )));
                },
            },
        };

        // Another writer can untrack the block while the request runs.
        let Some((stored_header, relevance)) = store.get_block_header_by_num(block_num).await?
        else {
            continue;
        };
        if !bool::from(relevance) {
            continue;
        }
        if stored_header != block_header {
            return Ok(Plan::Stop(format!(
                "the stored header of block {block_num} is not the header from the node"
            )));
        }

        match missing_nodes(&peaks_mmr, &nodes, &block_header, &mmr_proof, indices) {
            Ok(missing_nodes) => repairs.push(Repair { block_header, missing_nodes }),
            Err(reason) => return Ok(Plan::Stop(reason)),
        }
    }

    Ok(Plan::Store(repairs))
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

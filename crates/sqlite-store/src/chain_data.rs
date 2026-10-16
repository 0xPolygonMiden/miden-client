#![allow(clippy::items_after_statements)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Display;
use std::num::NonZeroUsize;
use std::vec::Vec;

use miden_client::Word;
use miden_client::block::BlockHeader;
use miden_client::crypto::{Forest, InOrderIndex, MmrPeaks, PartialMmr};
use miden_client::note::BlockNumber;
use miden_client::store::{
    BlockRelevance,
    NoteFilter,
    PartialBlockchainFilter,
    StoreError,
    build_partial_mmr,
    tracked_leaf_path_indices,
};
use miden_client::utils::{Deserializable, Serializable};
use rusqlite::{Connection, OptionalExtension, Transaction, params, params_from_iter};

use super::SqliteStore;
use crate::sql_error::SqlResultExt;
use crate::sync::query_sync_height;
use crate::{insert_sql, int_array, proto, subst, with_write_tx};

impl SqliteStore {
    pub(crate) fn get_block_headers(
        conn: &Connection,
        block_numbers: &BTreeSet<BlockNumber>,
    ) -> Result<Vec<(BlockHeader, BlockRelevance)>, StoreError> {
        let block_number_list =
            int_array(block_numbers.iter().map(|block_number| u64::from(block_number.as_u32())));

        const QUERY: &str =
            "SELECT header, has_client_notes FROM block_headers WHERE block_num IN rarray(?)";

        conn.prepare(QUERY)
            .into_store_error()?
            .query_map(params![block_number_list], parse_block_headers_columns)
            .into_store_error()?
            .map(|result| parse_block_header(result.into_store_error()?))
            .collect()
    }

    pub(crate) fn get_tracked_block_headers(
        conn: &Connection,
    ) -> Result<Vec<BlockHeader>, StoreError> {
        // `idx_block_headers_has_notes` is declared `WHERE has_client_notes = 1`, and SQLite
        // matches a partial index only when the predicate is spelled the same way.
        const QUERY: &str =
            "SELECT header, has_client_notes FROM block_headers WHERE has_client_notes=1";
        conn.prepare(QUERY)
            .into_store_error()?
            .query_map(params![], parse_block_headers_columns)
            .into_store_error()?
            .map(|result| parse_block_header(result.into_store_error()?).map(|(block, _)| block))
            .collect()
    }

    pub(crate) fn get_tracked_block_header_numbers(
        conn: &Connection,
    ) -> Result<BTreeSet<usize>, StoreError> {
        const QUERY: &str = "SELECT block_num FROM block_headers WHERE has_client_notes=1";
        conn.prepare(QUERY)
            .into_store_error()?
            .query_map(params![], |row| row.get::<_, u32>(0))
            .into_store_error()?
            .map(|result| {
                let block_num: u32 = result.into_store_error()?;
                Ok(block_num as usize)
            })
            .collect()
    }

    pub(crate) fn get_partial_blockchain_nodes(
        conn: &Connection,
        filter: &PartialBlockchainFilter,
    ) -> Result<BTreeMap<InOrderIndex, Word>, StoreError> {
        match filter {
            PartialBlockchainFilter::All => query_partial_blockchain_nodes(
                conn,
                "SELECT id, node FROM partial_blockchain_nodes",
                params![],
            ),

            PartialBlockchainFilter::List(ids) if ids.is_empty() => Ok(BTreeMap::new()),
            PartialBlockchainFilter::List(ids) => {
                let id_values = int_array(ids.iter().map(|id| id.inner() as u64));

                query_partial_blockchain_nodes(
                    conn,
                    "SELECT id, node FROM partial_blockchain_nodes WHERE id IN rarray(?)",
                    params_from_iter([id_values]),
                )
            },

            PartialBlockchainFilter::Forest(forest) if forest.is_empty() => Ok(BTreeMap::new()),
            PartialBlockchainFilter::Forest(forest) => {
                let max_index = i64::try_from(
                    forest.rightmost_in_order_index().expect("forest is not empty").inner(),
                )
                .expect("id is a valid i64");

                query_partial_blockchain_nodes(
                    conn,
                    "SELECT id, node FROM partial_blockchain_nodes WHERE id <= ?",
                    params![max_index],
                )
            },
        }
    }

    pub(crate) fn get_current_blockchain_peaks(conn: &Connection) -> Result<MmrPeaks, StoreError> {
        const QUERY: &str =
            "SELECT block_num, partial_blockchain_peaks FROM blockchain_checkpoint LIMIT 1";

        let (block_num, peaks_bytes): (u32, Vec<u8>) = conn
            .prepare(QUERY)
            .into_store_error()?
            .query_row(params![], |row| {
                Ok((row.get("block_num")?, row.get("partial_blockchain_peaks")?))
            })
            .into_store_error()?;

        // The seed row holds an empty blob, which decodes as an MMR without peaks.
        parse_partial_blockchain_peaks(block_num, &peaks_bytes)
    }

    /// Reads the parts of the current partial MMR in one transaction, so that a concurrent write
    /// cannot make them inconsistent, and builds the partial MMR from them.
    pub(crate) fn get_current_partial_mmr(conn: &mut Connection) -> Result<PartialMmr, StoreError> {
        let tx = conn.transaction().into_store_error()?;

        let peaks = Self::get_current_blockchain_peaks(&tx)?;
        let sync_height = BlockNumber::from(
            u32::try_from(peaks.num_leaves())
                .map_err(|err| StoreError::ParsingError(err.to_string()))?,
        );
        let (sync_header, relevance) = Self::get_block_headers(&tx, &[sync_height].into())?
            .pop()
            .ok_or(StoreError::BlockHeaderNotFound(sync_height))?;
        let tracked_blocks = Self::get_tracked_block_header_numbers(&tx)?;
        let mut forest = peaks.forest();
        forest.append_leaf().map_err(StoreError::MmrError)?;
        let nodes =
            Self::get_partial_blockchain_nodes(&tx, &PartialBlockchainFilter::Forest(forest))?;
        tx.commit().into_store_error()?;

        build_partial_mmr(peaks, &sync_header, relevance, tracked_blocks, nodes)
    }

    pub(crate) fn insert_block_header(
        conn: &mut Connection,
        block_header: &BlockHeader,
        nodes: &[(InOrderIndex, Word)],
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        with_write_tx(conn, |tx| {
            Self::insert_block_header_tx(tx, block_header, has_client_notes)?;
            Self::insert_partial_blockchain_nodes_tx(tx, nodes)?;
            // An insert only adds rows, so it cannot remove a node that another block needs.
            check_tracked_paths_tx(tx, TrackedPaths::Block(block_header.block_num()))
        })
    }

    /// Inserts a list of MMR authentication nodes to the Partial Blockchain nodes table.
    pub(crate) fn insert_partial_blockchain_nodes_tx(
        tx: &Transaction<'_>,
        nodes: &[(InOrderIndex, Word)],
    ) -> Result<(), StoreError> {
        const QUERY: &str = insert_sql!(partial_blockchain_nodes { id, node } | IGNORE);
        let mut stmt = tx.prepare_cached(QUERY).into_store_error()?;
        for (index, node) in nodes {
            stmt.execute(params![index.inner(), node.to_bytes()]).into_store_error()?;
        }
        Ok(())
    }

    /// Inserts a block header using a [`rusqlite::Transaction`].
    ///
    /// If the block header exists and `has_client_notes` is `true` then the `has_client_notes`
    /// column is updated to `true` to signify that the block now contains a relevant note.
    pub(crate) fn insert_block_header_tx(
        tx: &Transaction<'_>,
        block_header: &BlockHeader,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        const QUERY: &str =
            insert_sql!(block_headers { block_num, header, has_client_notes } | IGNORE);
        let block_num = block_header.block_num().as_u32();
        tx.execute(QUERY, params![block_num, proto::encode(block_header), has_client_notes])
            .into_store_error()?;

        set_block_header_has_client_notes(tx, u64::from(block_num), has_client_notes)?;
        Ok(())
    }

    /// Prunes irrelevant block data from the store.
    ///
    /// This performs three operations in a single transaction:
    /// 1. Deletes MMR authentication nodes at the given `node_indices`.
    /// 2. Sets `has_client_notes = false` for `blocks_to_untrack`.
    /// 3. Deletes block headers with `has_client_notes = false` that are not the genesis or
    ///    sync-height block.
    ///
    /// Refuses the prune as a stale chain write if an unspent input note has an inclusion proof for
    /// a block in `blocks_to_untrack`.
    pub(crate) fn untrack_and_prune_irrelevant_blocks(
        conn: &mut Connection,
        blocks_to_untrack: &[BlockNumber],
        node_indices_to_remove: &[InOrderIndex],
    ) -> Result<(), StoreError> {
        with_write_tx(conn, |tx| {
            // The caller chose the blocks from its own read of the unspent notes. Another writer
            // can store an unspent note in one of these blocks after that read.
            if let Some(block) = first_block_with_unspent_note(tx, blocks_to_untrack)? {
                return Err(stale_chain_write(format_args!(
                    "block {block} has an unspent input note and must stay tracked"
                )));
            }

            // 1. Delete stale MMR authentication nodes.
            let mut deleted_nodes = 0;
            if !node_indices_to_remove.is_empty() {
                let id_values =
                    int_array(node_indices_to_remove.iter().map(|id| id.inner() as u64));

                deleted_nodes = tx
                    .execute(
                        "DELETE FROM partial_blockchain_nodes WHERE id IN rarray(?)",
                        params![id_values],
                    )
                    .into_store_error()?;
            }

            // 2. Mark untracked blocks as irrelevant.
            if !blocks_to_untrack.is_empty() {
                let block_values =
                    int_array(blocks_to_untrack.iter().map(|b| u64::from(b.as_u32())));

                tx.execute(
                    "UPDATE block_headers SET has_client_notes = 0 WHERE block_num IN rarray(?)",
                    params![block_values],
                )
                .into_store_error()?;
            }

            // 3. Delete irrelevant block headers.
            let genesis: u32 = BlockNumber::GENESIS.as_u32();
            let sync_height = query_sync_height(tx)?.as_u32();

            tx.execute(
                "DELETE FROM block_headers \
                 WHERE has_client_notes = 0 \
                 AND block_num > ?1 \
                 AND block_num < ?2",
                rusqlite::params![genesis, sync_height],
            )
            .into_store_error()?;

            // The caller computes the nodes to delete from its own view of the partial MMR. If
            // another writer tracked a block after that read, the deleted nodes can be part of the
            // path of that block.
            if deleted_nodes > 0 {
                check_tracked_paths_tx(tx, TrackedPaths::All)?;
            }

            Ok(())
        })
    }
}

// TRACKED PATH CHECK
// ================================================================================================

/// The tracked blocks that [`check_tracked_paths_tx`] checks.
#[derive(Clone, Copy)]
pub(crate) enum TrackedPaths {
    /// Only the given block, and only if it is tracked.
    Block(BlockNumber),
    /// Every tracked block.
    All,
}

/// Refuses a write after which a tracked block does not have its leaf and its full path at the
/// forest that includes the sync-height block (see [`tracked_leaf_path_indices`]).
///
/// A store in that state cannot build its partial MMR, so every later sync and import fails. Run
/// this check inside the write transaction, after all of its writes, so that it sees the rows that
/// other connections committed before the transaction started.
///
/// # Errors
///
/// Returns [`StoreError::DatabaseTransientError`] that names the first block and node that are
/// missing. The caller can read the partial MMR again and retry the write.
pub(crate) fn check_tracked_paths_tx(
    conn: &Connection,
    scope: TrackedPaths,
) -> Result<(), StoreError> {
    let tracked_blocks: Vec<usize> = match scope {
        TrackedPaths::Block(block_num) => {
            let tracked: Option<bool> = conn
                .query_row(
                    "SELECT has_client_notes FROM block_headers WHERE block_num = ?",
                    params![block_num.as_u32()],
                    |row| row.get(0),
                )
                .optional()
                .into_store_error()?;
            if tracked != Some(true) {
                return Ok(());
            }
            vec![block_num.as_usize()]
        },
        TrackedPaths::All => {
            SqliteStore::get_tracked_block_header_numbers(conn)?.into_iter().collect()
        },
    };
    if tracked_blocks.is_empty() {
        return Ok(());
    }

    let sync_height = query_sync_height(conn)?;
    let mut required = Vec::new();
    for block in tracked_blocks {
        let Some(indices) = tracked_leaf_path_indices(sync_height, block) else {
            return Err(stale_chain_write(format_args!(
                "tracked block {block} is after the sync height {sync_height}"
            )));
        };
        required.extend(indices.into_iter().map(|index| (block, index)));
    }

    const QUERY: &str = "SELECT id FROM partial_blockchain_nodes WHERE id IN rarray(?)";
    let ids = int_array(required.iter().map(|(_, index)| index.inner() as u64));
    let stored: BTreeSet<usize> = conn
        .prepare_cached(QUERY)
        .into_store_error()?
        .query_map(params![ids], |row| row.get::<_, usize>(0))
        .into_store_error()?
        .collect::<Result<_, _>>()
        .into_store_error()?;

    match required.into_iter().find(|(_, index)| !stored.contains(&index.inner())) {
        Some((block, index)) => Err(stale_chain_write(format_args!(
            "tracked block {block} has no node at index {} for the sync height {sync_height}",
            index.inner()
        ))),
        None => Ok(()),
    }
}

pub(crate) fn stale_chain_write(reason: impl Display) -> StoreError {
    StoreError::DatabaseTransientError(format!("stale chain write: {reason}"))
}

/// Returns the first block of `blocks` for which an unspent input note has an inclusion proof.
fn first_block_with_unspent_note(
    conn: &Connection,
    blocks: &[BlockNumber],
) -> Result<Option<BlockNumber>, StoreError> {
    if blocks.is_empty() {
        return Ok(None);
    }
    let note_blocks: BTreeSet<BlockNumber> =
        SqliteStore::get_input_notes(conn, &NoteFilter::Unspent)?
            .iter()
            .filter_map(|note| note.inclusion_proof().map(|proof| proof.location().block_num()))
            .collect();
    Ok(blocks.iter().copied().find(|block| note_blocks.contains(block)))
}

// HELPERS
// ================================================================================================

fn query_partial_blockchain_nodes<P: rusqlite::Params>(
    conn: &Connection,
    sql: &str,
    params: P,
) -> Result<BTreeMap<InOrderIndex, Word>, StoreError> {
    let mut stmt = conn.prepare_cached(sql).into_store_error()?;

    stmt.query_map(params, |row| Ok((row.get::<_, usize>("id")?, row.get::<_, Vec<u8>>("node")?)))
        .into_store_error()?
        .map(|row_res| {
            let (id, node) = row_res.into_store_error()?;
            // An in-order index is never zero. A zero value means the row is corrupt.
            let id = NonZeroUsize::new(id).ok_or_else(|| {
                StoreError::ParsingError(
                    "stored partial blockchain node id must be non-zero".to_string(),
                )
            })?;
            Ok((InOrderIndex::new(id), Word::read_from_bytes(&node)?))
        })
        .collect()
}

fn parse_partial_blockchain_peaks(forest: u32, peaks_nodes: &[u8]) -> Result<MmrPeaks, StoreError> {
    let forest_size = usize::try_from(forest).expect("u64 should fit in usize");
    let forest = Forest::new(forest_size).map_err(|err| {
        StoreError::DataDeserializationError(
            miden_protocol::utils::serde::DeserializationError::InvalidValue(format!(
                "invalid forest size {forest_size}: {err}"
            )),
        )
    })?;
    Ok(proto::decode_mmr_peaks(forest, peaks_nodes)?)
}

fn parse_block_headers_columns(
    row: &rusqlite::Row<'_>,
) -> Result<(Vec<u8>, bool), rusqlite::Error> {
    Ok((row.get("header")?, row.get("has_client_notes")?))
}

fn parse_block_header(
    (header, has_client_notes): (Vec<u8>, bool),
) -> Result<(BlockHeader, BlockRelevance), StoreError> {
    Ok((proto::decode_unchecked(&header)?, has_client_notes.into()))
}

/// Marks the stored header of `block_num` as tracked if `has_client_notes` is `true`.
///
/// Returns whether the update moved the block from untracked to tracked. The update changes nothing
/// if the header is not stored.
pub(crate) fn set_block_header_has_client_notes(
    tx: &Transaction<'_>,
    block_num: u64,
    has_client_notes: bool,
) -> Result<bool, StoreError> {
    // Only update to change has_client_notes to true if it was false previously
    const QUERY: &str = "\
        UPDATE block_headers
        SET has_client_notes=?
        WHERE block_num=? AND has_client_notes=0;";
    let updated = tx.execute(QUERY, params![has_client_notes, block_num]).into_store_error()?;
    Ok(has_client_notes && updated > 0)
}

#[cfg(test)]
pub(crate) mod test {
    use std::collections::{BTreeMap, BTreeSet};
    use std::vec::Vec;

    use miden_client::Word;
    use miden_client::block::BlockHeader;
    use miden_client::crypto::{Forest, InOrderIndex, MmrPeaks};
    use miden_client::note::BlockNumber;
    use miden_client::store::{PartialBlockchainFilter, Store, StoreError};
    use miden_protocol::crypto::merkle::mmr::Mmr;
    use rusqlite::params;

    use crate::tests::create_test_store;
    use crate::{SqliteStore, proto};

    /// Returns `count` mock block headers and the chain MMR over their commitments.
    pub(crate) fn mock_chain(count: usize) -> (Vec<BlockHeader>, Mmr) {
        let headers: Vec<BlockHeader> = (0..count)
            .map(|n| BlockHeader::mock(u32::try_from(n).unwrap(), None, None, &[]))
            .collect();
        let mut mmr = Mmr::default();
        for header in &headers {
            mmr.add(header.commitment()).expect("valid MMR append");
        }
        (headers, mmr)
    }

    /// Moves the stored sync height to `height`: stores its header untracked and stores the peaks
    /// over the blocks below it.
    pub(crate) async fn set_sync_height(
        store: &SqliteStore,
        mmr: &Mmr,
        headers: &[BlockHeader],
        height: usize,
    ) {
        Store::insert_block_header(store, &headers[height], &[], false).await.unwrap();
        let peaks_bytes =
            proto::encode_mmr_peaks(&mmr.peaks_at(Forest::new(height).unwrap()).unwrap());
        let height = i64::try_from(height).unwrap();
        store
            .interact_with_connection(move |conn| {
                conn.execute(
                    "UPDATE blockchain_checkpoint SET block_num = ?, partial_blockchain_peaks = ?",
                    params![height, peaks_bytes],
                )
                .unwrap();
                Ok(())
            })
            .await
            .unwrap();
    }

    /// Returns the leaf of `block` followed by its authentication path at `forest`, in path order.
    pub(crate) fn path_nodes(
        mmr: &Mmr,
        headers: &[BlockHeader],
        block: usize,
        forest: Forest,
    ) -> Vec<(InOrderIndex, Word)> {
        let proof = mmr.open_at(block, forest).unwrap();
        let mut index = InOrderIndex::from_leaf_pos(block);
        let mut nodes = vec![(index, headers[block].commitment())];
        for node in proof.merkle_path().nodes() {
            nodes.push((index.sibling(), *node));
            index = index.parent();
        }
        nodes
    }

    async fn insert_dummy_block_headers(store: &mut SqliteStore) -> Vec<BlockHeader> {
        let block_headers: Vec<BlockHeader> =
            (0..5).map(|block_num| BlockHeader::mock(block_num, None, None, &[])).collect();

        let block_headers_clone = block_headers.clone();
        store
            .interact_with_connection(move |conn| {
                let tx = conn.transaction().unwrap();
                (0..5).for_each(|block_num| {
                    SqliteStore::insert_block_header_tx(
                        &tx,
                        &block_headers_clone[block_num],
                        false,
                    )
                    .unwrap();
                });
                tx.commit().unwrap();
                Ok(())
            })
            .await
            .unwrap();

        block_headers
    }

    #[tokio::test]
    async fn insert_and_get_block_headers_by_number() {
        let mut store = create_test_store().await;
        let block_headers = insert_dummy_block_headers(&mut store).await;

        let block_header = Store::get_block_header_by_num(&store, 3.into()).await.unwrap().unwrap();
        assert_eq!(block_headers[3], block_header.0);
    }

    #[tokio::test]
    async fn insert_and_get_block_headers_by_list() {
        let mut store = create_test_store().await;
        let mock_block_headers = insert_dummy_block_headers(&mut store).await;

        let block_headers: Vec<BlockHeader> =
            Store::get_block_headers(&store, &[1.into(), 3.into()].into_iter().collect())
                .await
                .unwrap()
                .into_iter()
                .map(|(block_header, _has_notes)| block_header)
                .collect();
        assert_eq!(
            &[mock_block_headers[1].clone(), mock_block_headers[3].clone()],
            &block_headers[..]
        );
    }

    /// Tests that `insert_block_header` persists the tracked header and its MMR authentication
    /// nodes in the same call, so both are retrievable afterwards.
    #[tokio::test]
    async fn insert_block_header_stores_header_and_nodes() {
        let store = create_test_store().await;
        const TOTAL_BLOCKS: usize = 8;

        let headers: Vec<BlockHeader> = (0..TOTAL_BLOCKS)
            .map(|n| BlockHeader::mock(u32::try_from(n).unwrap(), None, None, &[]))
            .collect();
        let mut mmr = Mmr::default();
        for header in &headers {
            mmr.add(header.commitment()).expect("valid MMR append");
        }

        // The paths are at forest 8, so the sync height must be block 7.
        set_sync_height(&store, &mmr, &headers, TOTAL_BLOCKS - 1).await;

        let tracked: BTreeSet<usize> = [5].into();
        let auth_nodes = collect_auth_nodes(&mmr, &headers, &tracked);
        let header = headers[5].clone();

        Store::insert_block_header(&store, &header, &auth_nodes, true).await.unwrap();

        // The header is stored and marked as tracked.
        let stored = Store::get_block_headers(&store, &[5.into()].into_iter().collect())
            .await
            .unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].0, header);

        let tracked = Store::get_tracked_block_header_numbers(&store).await.unwrap();
        assert_eq!(tracked, [5].into());

        // Every authentication node was stored by the same call.
        let stored_nodes =
            Store::get_partial_blockchain_nodes(&store, PartialBlockchainFilter::All)
                .await
                .unwrap();
        let expected: BTreeMap<InOrderIndex, Word> = auth_nodes.iter().copied().collect();
        assert_eq!(stored_nodes, expected);
    }

    /// Tests that a failure inserting the MMR nodes rolls back the block header written in the same
    /// call, proving both land in a single transaction.
    #[tokio::test]
    async fn insert_block_header_rolls_back_header_when_nodes_fail() {
        let store = create_test_store().await;
        let header = BlockHeader::mock(5, None, None, &[]);
        // One node so the node insert actually runs (an empty slice would be a no-op).
        let nodes = [(InOrderIndex::from_leaf_pos(5), header.commitment())];

        // Force the node insert (the second statement in the transaction) to fail.
        store
            .interact_with_connection(|conn| {
                conn.execute("DROP TABLE partial_blockchain_nodes", []).unwrap();
                Ok(())
            })
            .await
            .unwrap();

        let result = Store::insert_block_header(&store, &header, &nodes, true).await;
        assert!(result.is_err(), "node insert must fail against the dropped table");

        // The header must not survive: a non-atomic two-transaction insert would leave it behind.
        let stored = Store::get_block_headers(&store, &[5.into()].into_iter().collect())
            .await
            .unwrap();
        assert!(stored.is_empty(), "header must roll back when the node insert fails");
    }

    /// Tests that large stored MMRs are built consistently throughout multiple prunes
    #[tokio::test]
    async fn partial_mmr_reconstructs_after_multiple_prune() {
        // Setup (mock a large MMR to work with, with a partial tracked set)
        // ----------------------------------------------------------------------------------------

        let store = create_test_store().await;
        const TOTAL_BLOCKS: usize = 7300;

        let block_headers: Vec<BlockHeader> = (0..TOTAL_BLOCKS)
            .map(|block_num| BlockHeader::mock(u32::try_from(block_num).unwrap(), None, None, &[]))
            .collect();

        let mut mmr = Mmr::default();
        for header in &block_headers {
            mmr.add(header.commitment()).expect("valid MMR append");
        }

        let mut tracked_set: BTreeSet<usize> = (0..(TOTAL_BLOCKS - 1)).step_by(97).collect();
        tracked_set.insert(TOTAL_BLOCKS - 2);
        let tracked_blocks: Vec<usize> = tracked_set.iter().copied().collect();

        let mut tracked_nodes: BTreeMap<InOrderIndex, Word> = BTreeMap::new();
        for &block_num in &tracked_blocks {
            let header = &block_headers[block_num];
            tracked_nodes.insert(InOrderIndex::from_leaf_pos(block_num), header.commitment());

            let proof = mmr.open(block_num).expect("valid proof");
            let mut idx = InOrderIndex::from_leaf_pos(block_num);
            for node in proof.merkle_path().nodes() {
                tracked_nodes.insert(idx.sibling(), *node);
                idx = idx.parent();
            }
        }
        let tracked_nodes: Vec<(InOrderIndex, Word)> = tracked_nodes.into_iter().collect();

        let peaks_by_block: Vec<MmrPeaks> = (0..TOTAL_BLOCKS)
            .map(|block_num| {
                mmr.peaks_at(Forest::new(block_num).expect("valid forest"))
                    .expect("valid peaks")
            })
            .collect();

        // Save blocks and nodes
        store
            .interact_with_connection(move |conn| {
                let tx = conn.transaction().unwrap();
                for (block_num, block_header) in block_headers.iter().enumerate() {
                    let has_notes = tracked_set.contains(&block_num);
                    SqliteStore::insert_block_header_tx(&tx, block_header, has_notes).unwrap();
                }

                SqliteStore::insert_partial_blockchain_nodes_tx(&tx, &tracked_nodes).unwrap();
                tx.commit().unwrap();
                Ok(())
            })
            .await
            .unwrap();

        let prune_heights = [
            TOTAL_BLOCKS / 5,
            (TOTAL_BLOCKS * 2) / 5,
            (TOTAL_BLOCKS * 3) / 5,
            TOTAL_BLOCKS - 1,
        ];

        // Tests/assertions
        // ----------------------------------------------------------------------------------------

        let mut previous_remaining: Option<i64> = None;
        for height in prune_heights {
            let height_i64 = i64::try_from(height).expect("fits in i64");
            let peaks_bytes = proto::encode_mmr_peaks(&peaks_by_block[height]);

            // Update sync height (and the matching MMR peaks) to simulate having synced further
            store
                .interact_with_connection(move |conn| {
                    conn.execute(
                        "UPDATE blockchain_checkpoint SET block_num = ?, partial_blockchain_peaks = ?",
                        params![height_i64, peaks_bytes],
                    )
                    .unwrap();
                    Ok(())
                })
                .await
                .unwrap();

            // Prune
            store.untrack_and_prune_irrelevant_blocks(&[], &[]).await.unwrap();

            // Assert blocks
            let remaining_headers: i64 = store
                .interact_with_connection(|conn| {
                    let count = conn
                        .query_row("SELECT COUNT(*) FROM block_headers", [], |row| row.get(0))
                        .unwrap();
                    Ok(count)
                })
                .await
                .unwrap();
            if let Some(previous) = previous_remaining {
                assert!(remaining_headers < previous);
            } else {
                assert!(remaining_headers < i64::try_from(TOTAL_BLOCKS).unwrap());
            }
            previous_remaining = Some(remaining_headers);
        }

        // Try build MMR
        let partial_mmr = Store::get_current_partial_mmr(&store).await.unwrap();
        assert_eq!(partial_mmr.peaks().hash_peaks(), mmr.peaks().hash_peaks());

        for block_num in tracked_blocks {
            let partial_proof = partial_mmr.open(block_num).expect("partial mmr query succeeds");
            assert!(partial_proof.is_some());
            assert_eq!(
                partial_proof.unwrap().merkle_path(),
                mmr.open(block_num).unwrap().merkle_path()
            );
        }
    }

    /// Collects authentication nodes for a set of tracked leaves in an MMR.
    fn collect_auth_nodes(
        mmr: &Mmr,
        block_headers: &[BlockHeader],
        tracked: &BTreeSet<usize>,
    ) -> Vec<(InOrderIndex, Word)> {
        let mut nodes: BTreeMap<InOrderIndex, Word> = BTreeMap::new();
        for &block_num in tracked {
            nodes.insert(
                InOrderIndex::from_leaf_pos(block_num),
                block_headers[block_num].commitment(),
            );
            let proof = mmr.open(block_num).expect("valid proof");
            let mut idx = InOrderIndex::from_leaf_pos(block_num);
            for node in proof.merkle_path().nodes() {
                nodes.insert(idx.sibling(), *node);
                idx = idx.parent();
            }
        }
        nodes.into_iter().collect()
    }

    /// Tests that `untrack_and_prune_irrelevant_blocks` removes redundant authentication nodes for
    /// untracked blocks while preserving nodes needed by blocks that remain tracked.
    #[tokio::test]
    async fn prune_irrelevant_blocks_removes_redundant_auth_nodes() {
        let store = create_test_store().await;
        const TOTAL_BLOCKS: usize = 16;

        let headers: Vec<BlockHeader> = (0..TOTAL_BLOCKS)
            .map(|n| BlockHeader::mock(u32::try_from(n).unwrap(), None, None, &[]))
            .collect();
        let mut mmr = Mmr::default();
        for h in &headers {
            mmr.add(h.commitment()).expect("valid MMR append");
        }

        // Track blocks 3 and 10; we will untrack 3 later.
        let tracked: BTreeSet<usize> = [3, 10].into();
        let auth_nodes = collect_auth_nodes(&mmr, &headers, &tracked);
        let tip_peaks_bytes = proto::encode_mmr_peaks(
            &mmr.peaks_at(Forest::new(TOTAL_BLOCKS - 1).expect("valid forest")).unwrap(),
        );

        // Persist everything.
        let headers_clone = headers.clone();
        store
            .interact_with_connection(move |conn| {
                let tx = conn.transaction().unwrap();
                for (i, header) in headers_clone.iter().enumerate().take(TOTAL_BLOCKS) {
                    SqliteStore::insert_block_header_tx(&tx, header, tracked.contains(&i)).unwrap();
                }
                SqliteStore::insert_partial_blockchain_nodes_tx(&tx, &auth_nodes).unwrap();
                tx.execute(
                    "UPDATE blockchain_checkpoint SET block_num = ?, partial_blockchain_peaks = ?",
                    params![i64::try_from(TOTAL_BLOCKS - 1).unwrap(), tip_peaks_bytes],
                )
                .unwrap();
                tx.commit().unwrap();
                Ok(())
            })
            .await
            .unwrap();

        // Untrack block 3 via the PartialMmr, then prune.
        let mut partial_mmr = Store::get_current_partial_mmr(&store).await.unwrap();
        let removed: Vec<InOrderIndex> =
            partial_mmr.untrack(3).into_iter().map(|(idx, _)| idx).collect();
        assert!(!removed.is_empty(), "untracking should remove at least one node");

        store
            .untrack_and_prune_irrelevant_blocks(&[BlockNumber::from(3u32)], &removed)
            .await
            .unwrap();

        // Block 3 header should be deleted, block 10 should still be provable.
        let rebuilt = Store::get_current_partial_mmr(&store).await.unwrap();
        assert_eq!(rebuilt.peaks().hash_peaks(), mmr.peaks().hash_peaks());

        let proof_10 = rebuilt.open(10).expect("open succeeds");
        assert!(proof_10.is_some(), "block 10 should still be provable");

        let proof_3 = rebuilt.open(3).expect("open succeeds");
        assert!(proof_3.is_none(), "block 3 should no longer be provable");
    }

    /// Tests that the store refuses a tracked header whose path is too short for the forest at the
    /// stored sync height, and that it accepts a path whose missing levels are already stored.
    #[tokio::test]
    async fn insert_block_header_rejects_path_short_for_sync_height() {
        let store = create_test_store().await;
        let (headers, mmr) = mock_chain(40);
        set_sync_height(&store, &mmr, &headers, 39).await;

        // At forest 40, leaf 0 is in the tree of 32 leaves, so its path has 5 siblings.
        let forest = Forest::new(40).unwrap();
        let full_path = path_nodes(&mmr, &headers, 0, forest);
        assert_eq!(full_path.len(), 6);

        let result = Store::insert_block_header(&store, &headers[0], &full_path[..2], true).await;
        assert!(
            matches!(result, Err(StoreError::DatabaseTransientError(_))),
            "a path with 1 of 5 siblings must be refused, got {result:?}"
        );
        let stored = Store::get_block_headers(&store, &[0.into()].into_iter().collect())
            .await
            .unwrap();
        assert!(stored.is_empty(), "the refused header must not be stored");
        let nodes = Store::get_partial_blockchain_nodes(&store, PartialBlockchainFilter::All)
            .await
            .unwrap();
        assert!(nodes.is_empty(), "the refused nodes must not be stored");

        Store::insert_block_header(&store, &headers[0], &full_path, true).await.unwrap();

        // Blocks 0 and 1 share every sibling above level 0, so block 1 needs only its leaf and its
        // first sibling.
        let block_1_path = path_nodes(&mmr, &headers, 1, forest);
        Store::insert_block_header(&store, &headers[1], &block_1_path[..2], true)
            .await
            .unwrap();

        let partial_mmr = Store::get_current_partial_mmr(&store).await.unwrap();
        for block in [0, 1] {
            let proof = partial_mmr.open(block).unwrap().expect("block is tracked");
            assert_eq!(proof.merkle_path(), mmr.open_at(block, forest).unwrap().merkle_path());
        }
    }

    /// Tests that the store checks a tracked path at the forest that includes the sync-height
    /// block. At sync height 3, leaf 0 is in the tree of 4 leaves, so its level 0 sibling is not
    /// enough.
    #[tokio::test]
    async fn insert_block_header_checks_the_forest_of_the_sync_height_block() {
        let store = create_test_store().await;
        let (headers, mmr) = mock_chain(4);
        set_sync_height(&store, &mmr, &headers, 3).await;

        let full_path = path_nodes(&mmr, &headers, 0, Forest::new(4).unwrap());
        assert_eq!(full_path.len(), 3);
        let result = Store::insert_block_header(&store, &headers[0], &full_path[..2], true).await;
        assert!(
            matches!(result, Err(StoreError::DatabaseTransientError(_))),
            "a path with only the level 0 sibling must be refused, got {result:?}"
        );

        Store::insert_block_header(&store, &headers[0], &full_path, true).await.unwrap();
    }

    /// Tests that a prune computed from a view that predates a tracked insert is refused when it
    /// deletes nodes the inserted block needs.
    #[tokio::test]
    async fn prune_with_stale_nodes_is_rejected() {
        let store = create_test_store().await;
        let (headers, mmr) = mock_chain(16);
        set_sync_height(&store, &mmr, &headers, 15).await;
        let forest = Forest::new(16).unwrap();

        let stale_block = 4;
        let sibling_block = 5;
        Store::insert_block_header(
            &store,
            &headers[stale_block],
            &path_nodes(&mmr, &headers, stale_block, forest),
            true,
        )
        .await
        .unwrap();

        // The prune view sees only the stale block, so untracking it frees its whole path.
        let mut view = Store::get_current_partial_mmr(&store).await.unwrap();
        let stale_nodes: Vec<InOrderIndex> =
            view.untrack(stale_block).into_iter().map(|(index, _)| index).collect();

        // The sibling block shares that whole path.
        Store::insert_block_header(
            &store,
            &headers[sibling_block],
            &path_nodes(&mmr, &headers, sibling_block, forest),
            true,
        )
        .await
        .unwrap();

        let result = store
            .untrack_and_prune_irrelevant_blocks(&[BlockNumber::from(4u32)], &stale_nodes)
            .await;
        assert!(
            matches!(result, Err(StoreError::DatabaseTransientError(_))),
            "a prune that deletes a tracked path must be refused, got {result:?}"
        );

        let partial_mmr = Store::get_current_partial_mmr(&store).await.unwrap();
        assert!(partial_mmr.open(sibling_block).unwrap().is_some());
        assert!(partial_mmr.is_tracked(stale_block), "the refused prune must not untrack");
    }
}

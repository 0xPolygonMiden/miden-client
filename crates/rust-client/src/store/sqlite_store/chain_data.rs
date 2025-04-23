#![allow(clippy::items_after_statements)]

use alloc::{collections::BTreeMap, rc::Rc, string::String, vec::Vec};
use std::{collections::BTreeSet, num::NonZeroUsize};

use miden_objects::{
    Digest,
    block::{BlockHeader, BlockNumber},
    crypto::merkle::{InOrderIndex, MmrPeaks},
};
use miden_tx::utils::{Deserializable, Serializable};
use rusqlite::{
    Connection, OptionalExtension, Transaction, params, params_from_iter, types::Value,
};

use super::SqliteStore;
use crate::store::{ChainMmrNodeFilter, StoreError};

type SerializedBlockHeaderData = (u32, Vec<u8>, Vec<u8>, bool);
type SerializedBlockHeaderParts = (u64, Vec<u8>, Vec<u8>, bool);

type SerializedChainMmrNodeData = (i64, String);
type SerializedChainMmrNodeParts = (u64, String);

// CHAIN MMR NODE FILTER
// --------------------------------------------------------------------------------------------

impl ChainMmrNodeFilter {
    fn to_query(&self) -> String {
        let base = String::from("SELECT id, node FROM chain_mmr_nodes");
        match self {
            ChainMmrNodeFilter::All => base,
            ChainMmrNodeFilter::List(_) => {
                format!("{base} WHERE id IN rarray(?)")
            },
        }
    }
}

impl SqliteStore {
    pub(crate) fn insert_block_header(
        conn: &mut Connection,
        block_header: &BlockHeader,
        chain_mmr_peaks: &MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        let tx = conn.transaction()?;

        Self::insert_block_header_tx(&tx, block_header, chain_mmr_peaks, has_client_notes)?;

        tx.commit()?;
        Ok(())
    }

    pub(crate) fn get_block_headers(
        conn: &mut Connection,
        block_numbers: &BTreeSet<BlockNumber>,
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError> {
        let block_number_list = block_numbers
            .iter()
            .map(|block_number| Value::Integer(i64::from(block_number.as_u32())))
            .collect::<Vec<Value>>();

        const QUERY: &str = "SELECT block_num, header, chain_mmr_peaks, has_client_notes FROM block_headers WHERE block_num IN rarray(?)";

        conn.prepare(QUERY)?
            .query_map(params![Rc::new(block_number_list)], parse_block_headers_columns)?
            .map(|result| Ok(result?).and_then(parse_block_header))
            .collect()
    }

    pub(crate) fn get_tracked_block_headers(
        conn: &mut Connection,
    ) -> Result<Vec<BlockHeader>, StoreError> {
        const QUERY: &str = "SELECT block_num, header, chain_mmr_peaks, has_client_notes FROM block_headers WHERE has_client_notes=true";
        conn.prepare(QUERY)?
            .query_map(params![], parse_block_headers_columns)?
            .map(|result| Ok(result?).and_then(parse_block_header).map(|(block, _)| block))
            .collect()
    }

    pub(crate) fn get_chain_mmr_nodes(
        conn: &mut Connection,
        filter: &ChainMmrNodeFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        let mut params = Vec::new();
        if let ChainMmrNodeFilter::List(ids) = &filter {
            let id_values = ids
                .iter()
                 // SAFETY: d.inner() is a usize casted to u64, should not fail.
                .map(|id| Value::Integer(i64::try_from(id.inner()).expect("id is a valid i64")))
                .collect::<Vec<_>>();

            params.push(Rc::new(id_values));
        }

        conn.prepare(&filter.to_query())?
            .query_map(params_from_iter(params), parse_chain_mmr_nodes_columns)?
            .map(|result| Ok(result?).and_then(parse_chain_mmr_nodes))
            .collect()
    }

    pub(crate) fn get_chain_mmr_peaks_by_block_num(
        conn: &mut Connection,
        block_num: BlockNumber,
    ) -> Result<MmrPeaks, StoreError> {
        const QUERY: &str = "SELECT chain_mmr_peaks FROM block_headers WHERE block_num = ?";

        let mmr_peaks = conn
            .prepare(QUERY)?
            .query_row(params![block_num.as_u32()], |row| {
                let peaks: Vec<u8> = row.get(0)?;
                Ok(peaks)
            })
            .optional()?;

        if let Some(mmr_peaks) = mmr_peaks {
            return parse_mmr_peaks(block_num.as_u32(), &mmr_peaks);
        }

        Ok(MmrPeaks::new(0, vec![])?)
    }

    pub fn insert_chain_mmr_nodes(
        conn: &mut Connection,
        nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError> {
        let tx = conn.transaction()?;

        Self::insert_chain_mmr_nodes_tx(&tx, nodes)?;

        Ok(tx.commit().map(|_| ())?)
    }

    /// Inserts a list of MMR authentication nodes to the Chain MMR nodes table.
    pub(crate) fn insert_chain_mmr_nodes_tx(
        tx: &Transaction<'_>,
        nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError> {
        for (index, node) in nodes {
            insert_chain_mmr_node(tx, *index, *node)?;
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
        chain_mmr_peaks: &MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        let chain_mmr_peaks = chain_mmr_peaks.peaks().to_vec();
        let (block_num, header, chain_mmr, has_client_notes) =
            serialize_block_header(block_header, &chain_mmr_peaks, has_client_notes);
        const QUERY: &str = "\
        INSERT OR IGNORE INTO block_headers
            (block_num, header, chain_mmr_peaks, has_client_notes)
        VALUES (?, ?, ?, ?)";
        tx.execute(QUERY, params![block_num, header, chain_mmr, has_client_notes])?;

        set_block_header_has_client_notes(tx, u64::from(block_num), has_client_notes)?;
        Ok(())
    }
}

// HELPERS
// ================================================================================================

/// Inserts a node represented by its in-order index and the node value.
fn insert_chain_mmr_node(
    tx: &Transaction<'_>,
    id: InOrderIndex,
    node: Digest,
) -> Result<(), StoreError> {
    let (id, node) = serialize_chain_mmr_node(id, node);
    const QUERY: &str = "INSERT OR IGNORE INTO chain_mmr_nodes (id, node) VALUES (?, ?)";
    tx.execute(QUERY, params![id, node])?;
    Ok(())
}

fn parse_mmr_peaks(forest: u32, peaks_nodes: &[u8]) -> Result<MmrPeaks, StoreError> {
    let mmr_peaks_nodes = Vec::<Digest>::read_from_bytes(peaks_nodes)?;

    MmrPeaks::new(forest as usize, mmr_peaks_nodes).map_err(StoreError::MmrError)
}

fn serialize_block_header(
    block_header: &BlockHeader,
    chain_mmr_peaks: &[Digest],
    has_client_notes: bool,
) -> SerializedBlockHeaderData {
    let block_num = block_header.block_num();
    let header = block_header.to_bytes();
    let chain_mmr_peaks = chain_mmr_peaks.to_bytes();

    (block_num.as_u32(), header, chain_mmr_peaks, has_client_notes)
}

fn parse_block_headers_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedBlockHeaderParts, rusqlite::Error> {
    let block_num: u32 = row.get(0)?;
    let header: Vec<u8> = row.get(1)?;
    let chain_mmr: Vec<u8> = row.get(2)?;
    let has_client_notes: bool = row.get(3)?;

    Ok((u64::from(block_num), header, chain_mmr, has_client_notes))
}

fn parse_block_header(
    serialized_block_header_parts: SerializedBlockHeaderParts,
) -> Result<(BlockHeader, bool), StoreError> {
    let (_, header, _, has_client_notes) = serialized_block_header_parts;

    Ok((BlockHeader::read_from_bytes(&header)?, has_client_notes))
}

fn serialize_chain_mmr_node(id: InOrderIndex, node: Digest) -> SerializedChainMmrNodeData {
    let id = i64::try_from(id.inner()).expect("id is a valid i64");
    let node = node.to_hex();
    (id, node)
}

fn parse_chain_mmr_nodes_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedChainMmrNodeParts, rusqlite::Error> {
    let id: u64 = row.get(0)?;
    let node = row.get(1)?;
    Ok((id, node))
}

fn parse_chain_mmr_nodes(
    serialized_chain_mmr_node_parts: SerializedChainMmrNodeParts,
) -> Result<(InOrderIndex, Digest), StoreError> {
    let (id, node) = serialized_chain_mmr_node_parts;

    let id = InOrderIndex::new(
        NonZeroUsize::new(usize::try_from(id).expect("id is u64, should not fail")).unwrap(),
    );
    let node: Digest = Digest::try_from(&node)?;
    Ok((id, node))
}

fn set_block_header_has_client_notes(
    tx: &Transaction<'_>,
    block_num: u64,
    has_client_notes: bool,
) -> Result<(), StoreError> {
    // Only update to change has_client_notes to true if it was false previously
    const QUERY: &str = "\
    UPDATE block_headers
        SET has_client_notes=?
        WHERE block_num=? AND has_client_notes=FALSE;";
    tx.execute(QUERY, params![has_client_notes, block_num])?;
    Ok(())
}

#[cfg(test)]
mod test {
    use alloc::vec::Vec;

    use miden_lib::transaction::TransactionKernel;
    use miden_objects::{block::BlockHeader, crypto::merkle::MmrPeaks};

    use crate::store::{
        Store,
        sqlite_store::{SqliteStore, tests::create_test_store},
    };

    async fn insert_dummy_block_headers(store: &mut SqliteStore) -> Vec<BlockHeader> {
        let block_headers: Vec<BlockHeader> = (0..5)
            .map(|block_num| {
                BlockHeader::mock(
                    block_num,
                    None,
                    None,
                    &[],
                    TransactionKernel::kernel_commitment(),
                )
            })
            .collect();

        let block_headers_clone = block_headers.clone();
        store
            .interact_with_connection(move |conn| {
                let tx = conn.transaction().unwrap();
                let dummy_peaks = MmrPeaks::new(0, Vec::new()).unwrap();
                (0..5).for_each(|block_num| {
                    SqliteStore::insert_block_header_tx(
                        &tx,
                        &block_headers_clone[block_num],
                        &dummy_peaks,
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
}

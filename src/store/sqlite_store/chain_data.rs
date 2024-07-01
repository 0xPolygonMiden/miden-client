use alloc::{collections::BTreeMap, rc::Rc};
use std::num::NonZeroUsize;

use miden_objects::{
    crypto::merkle::{InOrderIndex, MmrPeaks},
    BlockHeader, Digest,
};
use rusqlite::{params, params_from_iter, types::Value, OptionalExtension, Transaction};

use super::SqliteStore;
use crate::store::{ChainMmrNodeFilter, StoreError};

type SerializedBlockHeaderData = (i64, String, String, bool);
type SerializedBlockHeaderParts = (u64, String, String, bool);

type SerializedChainMmrNodeData = (i64, String);
type SerializedChainMmrNodeParts = (u64, String);

// CHAIN MMR NODE FILTER
// --------------------------------------------------------------------------------------------

impl ChainMmrNodeFilter<'_> {
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
        &self,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        let mut db = self.db();
        let tx = db.transaction()?;

        Self::insert_block_header_tx(&tx, block_header, chain_mmr_peaks, has_client_notes)?;

        tx.commit()?;
        Ok(())
    }

    pub(crate) fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError> {
        let block_number_list = block_numbers
            .iter()
            .map(|block_number| Value::Integer(*block_number as i64))
            .collect::<Vec<Value>>();

        const QUERY : &str = "SELECT block_num, header, chain_mmr_peaks, has_client_notes FROM block_headers WHERE block_num IN rarray(?)";

        self.db()
            .prepare(QUERY)?
            .query_map(params![Rc::new(block_number_list)], parse_block_headers_columns)?
            .map(|result| Ok(result?).and_then(parse_block_header))
            .collect()
    }

    pub(crate) fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError> {
        const QUERY: &str = "SELECT block_num, header, chain_mmr_peaks, has_client_notes FROM block_headers WHERE has_client_notes=true";
        self.db()
            .prepare(QUERY)?
            .query_map(params![], parse_block_headers_columns)?
            .map(|result| Ok(result?).and_then(parse_block_header).map(|(block, _)| block))
            .collect()
    }

    pub(crate) fn get_chain_mmr_nodes(
        &self,
        filter: ChainMmrNodeFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        let mut params = Vec::new();
        if let ChainMmrNodeFilter::List(ids) = filter {
            let id_values = ids
                .iter()
                .map(|id| Value::Integer(Into::<u64>::into(*id) as i64))
                .collect::<Vec<_>>();

            params.push(Rc::new(id_values));
        }

        self.db()
            .prepare(&filter.to_query())?
            .query_map(params_from_iter(params), parse_chain_mmr_nodes_columns)?
            .map(|result| Ok(result?).and_then(parse_chain_mmr_nodes))
            .collect()
    }

    pub(crate) fn get_chain_mmr_peaks_by_block_num(
        &self,
        block_num: u32,
    ) -> Result<MmrPeaks, StoreError> {
        const QUERY: &str = "SELECT chain_mmr_peaks FROM block_headers WHERE block_num = ?";

        let mmr_peaks = self
            .db()
            .prepare(QUERY)?
            .query_row(params![block_num], |row| {
                let peaks: String = row.get(0)?;
                Ok(peaks)
            })
            .optional()?;

        if let Some(mmr_peaks) = mmr_peaks {
            return parse_mmr_peaks(block_num, mmr_peaks);
        }

        Ok(MmrPeaks::new(0, vec![])?)
    }

    pub fn insert_chain_mmr_nodes(
        &self,
        nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError> {
        let mut db = self.db();
        let tx = db.transaction()?;

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

    /// Inserts a block header using a [rusqlite::Transaction]
    pub(crate) fn insert_block_header_tx(
        tx: &Transaction<'_>,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        let chain_mmr_peaks = chain_mmr_peaks.peaks().to_vec();
        let (block_num, header, chain_mmr, has_client_notes) =
            serialize_block_header(block_header, chain_mmr_peaks, has_client_notes)?;
        const QUERY: &str = "\
        INSERT OR IGNORE INTO block_headers
            (block_num, header, chain_mmr_peaks, has_client_notes)
        VALUES (?, ?, ?, ?)";
        tx.execute(QUERY, params![block_num, header, chain_mmr, has_client_notes])?;
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
    let (id, node) = serialize_chain_mmr_node(id, node)?;
    const QUERY: &str = "INSERT OR IGNORE INTO chain_mmr_nodes (id, node) VALUES (?, ?)";
    tx.execute(QUERY, params![id, node])?;
    Ok(())
}

fn parse_mmr_peaks(forest: u32, peaks_nodes: String) -> Result<MmrPeaks, StoreError> {
    let mmr_peaks_nodes: Vec<Digest> =
        serde_json::from_str(&peaks_nodes).map_err(StoreError::JsonDataDeserializationError)?;

    MmrPeaks::new(forest as usize, mmr_peaks_nodes).map_err(StoreError::MmrError)
}

fn serialize_block_header(
    block_header: BlockHeader,
    chain_mmr_peaks: Vec<Digest>,
    has_client_notes: bool,
) -> Result<SerializedBlockHeaderData, StoreError> {
    let block_num = block_header.block_num();
    let header =
        serde_json::to_string(&block_header).map_err(StoreError::InputSerializationError)?;
    let chain_mmr_peaks =
        serde_json::to_string(&chain_mmr_peaks).map_err(StoreError::InputSerializationError)?;

    Ok((block_num as i64, header, chain_mmr_peaks, has_client_notes))
}

fn parse_block_headers_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedBlockHeaderParts, rusqlite::Error> {
    let block_num: i64 = row.get(0)?;
    let header: String = row.get(1)?;
    let chain_mmr: String = row.get(2)?;
    let has_client_notes: bool = row.get(3)?;

    Ok((block_num as u64, header, chain_mmr, has_client_notes))
}

fn parse_block_header(
    serialized_block_header_parts: SerializedBlockHeaderParts,
) -> Result<(BlockHeader, bool), StoreError> {
    let (_, header, _, has_client_notes) = serialized_block_header_parts;

    Ok((
        serde_json::from_str(&header).map_err(StoreError::JsonDataDeserializationError)?,
        has_client_notes,
    ))
}

fn serialize_chain_mmr_node(
    id: InOrderIndex,
    node: Digest,
) -> Result<SerializedChainMmrNodeData, StoreError> {
    let id: u64 = id.into();
    let node = serde_json::to_string(&node).map_err(StoreError::InputSerializationError)?;
    Ok((id as i64, node))
}

fn parse_chain_mmr_nodes_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedChainMmrNodeParts, rusqlite::Error> {
    let id: i64 = row.get(0)?;
    let node = row.get(1)?;
    Ok((id as u64, node))
}

fn parse_chain_mmr_nodes(
    serialized_chain_mmr_node_parts: SerializedChainMmrNodeParts,
) -> Result<(InOrderIndex, Digest), StoreError> {
    let (id, node) = serialized_chain_mmr_node_parts;

    let id = InOrderIndex::new(NonZeroUsize::new(id as usize).unwrap());
    let node: Digest =
        serde_json::from_str(&node).map_err(StoreError::JsonDataDeserializationError)?;
    Ok((id, node))
}

#[cfg(test)]
mod test {
    use miden_objects::{crypto::merkle::MmrPeaks, BlockHeader};

    use crate::store::{
        sqlite_store::{tests::create_test_store, SqliteStore},
        Store,
    };

    fn insert_dummy_block_headers(store: &mut SqliteStore) -> Vec<BlockHeader> {
        let block_headers: Vec<BlockHeader> =
            (0..5).map(|block_num| BlockHeader::mock(block_num, None, None, &[])).collect();
        let mut db = store.db();
        let tx = db.transaction().unwrap();
        let dummy_peaks = MmrPeaks::new(0, Vec::new()).unwrap();
        (0..5).for_each(|block_num| {
            SqliteStore::insert_block_header_tx(
                &tx,
                block_headers[block_num],
                dummy_peaks.clone(),
                false,
            )
            .unwrap()
        });
        tx.commit().unwrap();

        block_headers
    }

    #[tokio::test]
    async fn insert_and_get_block_headers_by_number() {
        let mut store = create_test_store();
        let block_headers = insert_dummy_block_headers(&mut store);

        let block_header =
            winter_maybe_async::maybe_await!(store.get_block_header_by_num(3)).unwrap();
        assert_eq!(block_headers[3], block_header.0);
    }

    #[test]
    fn insert_and_get_block_headers_by_list() {
        let mut store = create_test_store();
        let mock_block_headers = insert_dummy_block_headers(&mut store);

        let block_headers: Vec<BlockHeader> = store
            .get_block_headers(&[1, 3])
            .unwrap()
            .into_iter()
            .map(|(block_header, _has_notes)| block_header)
            .collect();
        assert_eq!(&[mock_block_headers[1], mock_block_headers[3]], &block_headers[..]);
    }
}

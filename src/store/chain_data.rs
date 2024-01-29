use std::num::NonZeroUsize;

use super::Store;
use crate::errors::StoreError;
use clap::error::Result;

use crypto::merkle::{InOrderIndex, MerklePath, MmrPeaks};

use objects::utils::collections::{BTreeMap, BTreeSet};
use objects::{BlockHeader, Digest};
use rusqlite::{params, OptionalExtension, Transaction};
type SerializedBlockHeaderData = (i64, String, String, String, String, bool);
type SerializedBlockHeaderParts = (u64, String, String, String, String, bool);

type SerializedChainMmrNodeData = (i64, String);
type SerializedChainMmrNodeParts = (u64, String);

pub enum ChainMmrNodeFilter<'a> {
    All,
    List(&'a [InOrderIndex]),
}

impl ChainMmrNodeFilter<'_> {
    pub fn to_query(&self) -> String {
        let base = String::from("SELECT id, node FROM chain_mmr_nodes");
        match self {
            ChainMmrNodeFilter::All => base,
            ChainMmrNodeFilter::List(ids) => {
                let formatted_list = ids
                    .iter()
                    .map(|id| (Into::<u64>::into(*id)).to_string())
                    .collect::<Vec<String>>()
                    .join(",");
                format!("{base} WHERE id IN ({})", formatted_list)
            }
        }
    }
}

impl Store {
    // CHAIN DATA
    // --------------------------------------------------------------------------------------------

    /// Inserts a block header into the store, alongside peaks information at the block's height.
    ///
    /// `has_client_notes` describes whether the block has relevant notes to the client; this means
    /// the client might want to authenticate merkle paths based on this value.
    pub fn insert_block_header(
        tx: &Transaction<'_>,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        let chain_mmr_peaks = chain_mmr_peaks.peaks().to_vec();
        let (block_num, header, notes_root, sub_hash, chain_mmr, has_client_notes) =
            serialize_block_header(block_header, chain_mmr_peaks, has_client_notes)?;

        const QUERY: &str = "\
        INSERT INTO block_headers
            (block_num, header, notes_root, sub_hash, chain_mmr_peaks, has_client_notes)
         VALUES (?, ?, ?, ?, ?, ?)";

        tx.execute(
            QUERY,
            params![
                block_num,
                header,
                notes_root,
                sub_hash,
                chain_mmr,
                has_client_notes
            ],
        )
        .map_err(StoreError::QueryError)
        .map(|_| ())
    }

    /// Retrieves a [BlockHeader] by number and a boolean value that represents whether the
    /// block contains notes relevant to the client.
    pub fn get_block_header_by_num(
        &self,
        block_number: u32,
    ) -> Result<(BlockHeader, bool), StoreError> {
        const QUERY: &str = "SELECT block_num, header, notes_root, sub_hash, chain_mmr_peaks, has_client_notes FROM block_headers WHERE block_num = ?";
        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map(params![block_number as i64], parse_block_headers_columns)
            .map_err(StoreError::QueryError)?
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_block_header)
            })
            .next()
            .ok_or(StoreError::BlockHeaderNotFound(block_number))?
    }

    /// Retrieves a list of [BlockHeader] that include relevant notes to the client.
    pub fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError> {
        const QUERY: &str = "SELECT block_num, header, notes_root, sub_hash, chain_mmr_peaks, has_client_notes FROM block_headers WHERE has_client_notes=true";
        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map(params![], parse_block_headers_columns)
            .map_err(StoreError::QueryError)?
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_block_header)
                    .map(|(block, _had_notes)| block)
            })
            .collect::<Result<Vec<BlockHeader>, _>>()
    }

    /// Inserts a node represented by its in-order index and the node value.
    fn insert_chain_mmr_node(
        tx: &Transaction<'_>,
        id: InOrderIndex,
        node: Digest,
    ) -> Result<(), StoreError> {
        let (id, node) = serialize_chain_mmr_node(id, node)?;

        const QUERY: &str = "INSERT INTO chain_mmr_nodes (id, node) VALUES (?, ?)";

        tx.execute(QUERY, params![id, node])
            .map_err(StoreError::QueryError)
            .map(|_| ())
    }

    /// Inserts a list of MMR authentication nodes to the Chain MMR nodes table.
    pub(super) fn insert_chain_mmr_nodes(
        tx: &Transaction<'_>,
        nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError> {
        for (index, node) in nodes {
            Self::insert_chain_mmr_node(tx, *index, *node)?;
        }

        Ok(())
    }

    /// Retrieves all MMR authentication nodes based on [ChainMmrNodeFilter].
    pub fn get_chain_mmr_nodes(
        &self,
        filter: ChainMmrNodeFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        self.db
            .prepare(&filter.to_query())
            .map_err(StoreError::QueryError)?
            .query_map(params![], parse_chain_mmr_nodes_columns)
            .map_err(StoreError::QueryError)?
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_chain_mmr_nodes)
            })
            .collect()
    }

    /// Returns peaks information from the blockchain by a specific block number.
    pub fn get_chain_mmr_peaks_by_block_num(&self, block_num: u32) -> Result<MmrPeaks, StoreError> {
        const QUERY: &str = "SELECT chain_mmr_peaks FROM block_headers WHERE block_num = ?";

        let mmr_peaks = self
            .db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_row(params![block_num], |row| {
                let peaks: String = row.get(0)?;
                Ok(peaks)
            })
            .optional()
            .map_err(StoreError::QueryError)?;

        if let Some(mmr_peaks) = mmr_peaks {
            return parse_mmr_peaks(block_num, mmr_peaks);
        }

        MmrPeaks::new(0, vec![]).map_err(StoreError::MmrError)
    }

    /// Retrieves all Chain MMR nodes required for authenticating the set of blocks, and then
    /// constructs the path for each of them.
    ///
    /// This method assumes `block_nums` cannot contain `forest`.
    pub fn get_authentication_path_for_blocks(
        &self,
        block_nums: &[u32],
        forest: usize,
    ) -> Result<Vec<MerklePath>, StoreError> {
        let mut node_indices = BTreeSet::new();

        // Calculate all needed nodes indices for generating the paths
        for block_num in block_nums {
            let block_num = *block_num as usize;
            let before = forest & block_num;
            let after = forest ^ before;
            let path_depth = after.ilog2() as usize;

            let mut idx = InOrderIndex::from_leaf_pos(block_num);

            for _ in 0..path_depth {
                node_indices.insert(idx.sibling());
                idx = idx.parent();
            }
        }

        // Get all Mmr nodes based on collected indices
        let node_indices: Vec<InOrderIndex> = node_indices.into_iter().collect();

        let filter = ChainMmrNodeFilter::List(&node_indices);
        let mmr_nodes = self.get_chain_mmr_nodes(filter)?;

        // Construct authentication paths
        let mut authentication_paths = vec![];
        for block_num in block_nums {
            let mut merkle_nodes = vec![];
            let mut idx = InOrderIndex::from_leaf_pos(*block_num as usize);

            while let Some(node) = mmr_nodes.get(&idx.sibling()) {
                merkle_nodes.push(*node);
                idx = idx.parent();
            }
            let path = MerklePath::new(merkle_nodes);
            authentication_paths.push(path);
        }

        Ok(authentication_paths)
    }
}

// HELPERS
// ================================================================================================

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
    let notes_root = serde_json::to_string(&block_header.note_root())
        .map_err(StoreError::InputSerializationError)?;
    let sub_hash = serde_json::to_string(&block_header.sub_hash())
        .map_err(StoreError::InputSerializationError)?;
    let chain_mmr_peaks =
        serde_json::to_string(&chain_mmr_peaks).map_err(StoreError::InputSerializationError)?;

    Ok((
        block_num as i64,
        header,
        notes_root,
        sub_hash,
        chain_mmr_peaks,
        has_client_notes,
    ))
}

fn parse_block_headers_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedBlockHeaderParts, rusqlite::Error> {
    let block_num: i64 = row.get(0)?;
    let header: String = row.get(1)?;
    let notes_root: String = row.get(2)?;
    let sub_hash: String = row.get(3)?;
    let chain_mmr: String = row.get(4)?;
    let has_client_notes: bool = row.get(5)?;

    Ok((
        block_num as u64,
        header,
        notes_root,
        sub_hash,
        chain_mmr,
        has_client_notes,
    ))
}

fn parse_block_header(
    serialized_block_header_parts: SerializedBlockHeaderParts,
) -> Result<(BlockHeader, bool), StoreError> {
    let (_, header, _, _, _, has_client_notes) = serialized_block_header_parts;

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

use std::{collections::BTreeMap, num::NonZeroUsize};

use super::Store;

use crate::errors::StoreError;

use clap::error::Result;

use crypto::merkle::{InOrderIndex, MmrPeaks};
use objects::{BlockHeader, Digest};
use rusqlite::{params, OptionalExtension, Transaction};

type SerializedBlockHeaderData = (i64, String, String, String, String);
type SerializedBlockHeaderParts = (u64, String, String, String, String);

type SerializedChainMmrNodeData = (i64, String);
type SerializedChainMmrNodeParts = (u64, String);

// Block FILTER
// ================================================================================================
/// Represents a filter for blocks
pub enum BlockFilter {
    All,
    Single(u32),
    Range(u32, u32), // Represents inclusive range [start, end]
    List(Vec<u32>),
}

impl BlockFilter {
    pub fn to_query_filter(&self) -> String {
        match self {
            BlockFilter::All => String::from(""),
            BlockFilter::Single(block_height) => {
                format!("WHERE block_number = {}", *block_height as i64)
            }
            BlockFilter::Range(start, end) => format!(
                "WHERE block_number >= {} AND block_number < {}",
                *start as i64, *end as i64
            ),
            BlockFilter::List(block_numbers) => {
                let block_numbers_condition = block_numbers
                    .iter()
                    .map(|block_number| format!("block_number = {}", *block_number as i64))
                    .collect::<Vec<String>>()
                    .join(" AND ");
                format!("WHERE {}", block_numbers_condition)
            }
        }
    }
}

impl Store {
    // CHAIN DATA
    // --------------------------------------------------------------------------------------------
    pub fn insert_block_header(
        tx: &Transaction<'_>,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
    ) -> Result<(), StoreError> {
        let chain_mmr_peaks = chain_mmr_peaks.peaks().to_vec();
        let (block_num, header, notes_root, sub_hash, chain_mmr) =
            serialize_block_header(block_header, chain_mmr_peaks)?;

        const QUERY: &str = "\
        INSERT INTO block_headers
            (block_num, header, notes_root, sub_hash, chain_mmr_peaks)
         VALUES (?, ?, ?, ?, ?)";

        tx.execute(
            QUERY,
            params![block_num, header, notes_root, sub_hash, chain_mmr],
        )
        .map_err(StoreError::QueryError)
        .map(|_| ())
    }

    pub fn get_block_headers(&self, filter: BlockFilter) -> Result<Vec<BlockHeader>, StoreError> {
        let query = format!(
            "SELECT block_num, header, notes_root, sub_hash, chain_mmr_peaks FROM block_headers {}",
            filter.to_query_filter()
        );
        self.db
            .prepare(&query)
            .map_err(StoreError::QueryError)?
            .query_map(params![], parse_block_headers_columns)
            .map_err(StoreError::QueryError)?
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_block_header)
            })
            .collect()
    }

    pub fn get_block_header_by_num(&self, block_number: u32) -> Result<BlockHeader, StoreError> {
        self.get_block_headers(BlockFilter::Single(block_number))?
            .first()
            .copied()
            .ok_or(StoreError::BlockHeaderNotFound(block_number))
    }

    pub(crate) fn insert_chain_mmr_node(
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

    pub fn insert_chain_mmr_nodes(
        tx: &Transaction<'_>,
        nodes: Vec<(InOrderIndex, Digest)>,
    ) -> Result<(), StoreError> {
        for (index, node) in nodes {
            Self::insert_chain_mmr_node(tx, index, node)?;
        }

        Ok(())
    }

    /// Returns all nodes in the table.
    pub fn get_chain_mmr_nodes(&self) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        const QUERY: &str = "SELECT id, node FROM chain_mmr_nodes";
        self.db
            .prepare(QUERY)
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

    Ok((block_num as u64, header, notes_root, sub_hash, chain_mmr))
}

fn parse_block_header(
    serialized_block_header_parts: SerializedBlockHeaderParts,
) -> Result<BlockHeader, StoreError> {
    let (_, header, _, _, _) = serialized_block_header_parts;

    serde_json::from_str(&header).map_err(StoreError::JsonDataDeserializationError)
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

use std::{collections::BTreeMap, num::NonZeroUsize};

use super::Store;

use crate::errors::StoreError;

use clap::error::Result;

use crypto::merkle::InOrderIndex;
use objects::{BlockHeader, Digest};
use rusqlite::{params, Transaction};

type SerializedBlockHeaderData = (i64, String, String, String, String);
type SerializedBlockHeaderParts = (i64, String, String, String, String);

type SerializedChainMmrNodeData = (i64, String);
type SerializedChainMmrNodeParts = (i64, String);

impl Store {
    // CHAIN DATA
    // --------------------------------------------------------------------------------------------
    pub fn insert_block_header(
        tx: &Transaction<'_>,
        block_header: BlockHeader,
        chain_mmr_peaks: Vec<Digest>,
    ) -> Result<(), StoreError> {
        let (block_num, header, notes_root, sub_hash, chain_mmr) =
            serialize_block_header(block_header, chain_mmr_peaks)?;

        const QUERY: &str = "\
        INSERT INTO block_headers
            (block_num, header, notes_root, sub_hash, chain_mmr)
         VALUES (?, ?, ?, ?, ?)";

        tx.execute(
            QUERY,
            params![block_num, header, notes_root, sub_hash, chain_mmr],
        )
        .map_err(StoreError::QueryError)
        .map(|_| ())?;
        todo!() // TODO: pending changes https://github.com/0xPolygonMiden/miden-client/pull/63#discussion_r1432892042
    }

    #[cfg(test)]
    pub fn get_block_header_by_num(&self, block_number: u32) -> Result<BlockHeader, StoreError> {
        const QUERY: &str = "SELECT block_num, header, notes_root, sub_hash, chain_mmr FROM block_headers WHERE block_num = ?";
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
    pub fn get_chain_mmr_nodes(
        tx: &Transaction<'_>,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        const QUERY: &str = "SELECT id, node FROM chain_mmr_nodes";
        tx.prepare(QUERY)
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
}

// HELPERS
// ================================================================================================

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

// Unused until we need to query the block headers table
#[allow(dead_code)]
fn parse_block_headers_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedBlockHeaderParts, rusqlite::Error> {
    let block_num: i64 = row.get(0)?;
    let header: String = row.get(1)?;
    let notes_root: String = row.get(2)?;
    let sub_hash: String = row.get(3)?;
    let chain_mmr: String = row.get(4)?;
    Ok((block_num, header, notes_root, sub_hash, chain_mmr))
}

// Unused until we need to query the block headers table
#[allow(dead_code)]
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
    let id = row.get(0)?;
    let node = row.get(1)?;
    Ok((id, node))
}

fn parse_chain_mmr_nodes(
    serialized_chain_mmr_node_parts: SerializedChainMmrNodeParts,
) -> Result<(InOrderIndex, Digest), StoreError> {
    let (id, node) = serialized_chain_mmr_node_parts;

    let id = InOrderIndex::new(NonZeroUsize::new((id as u64) as usize).unwrap());
    let node: Digest =
        serde_json::from_str(&node).map_err(StoreError::JsonDataDeserializationError)?;
    Ok((id, node))
}

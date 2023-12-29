use objects::{notes::NoteInclusionProof, Digest};
use rusqlite::params;

use crate::errors::StoreError;

use super::Store;

impl Store {
    // STATE SYNC
    // --------------------------------------------------------------------------------------------

    /// Returns the note tags that the client is interested in.
    pub fn get_note_tags(&self) -> Result<Vec<u64>, StoreError> {
        const QUERY: &str = "SELECT tags FROM state_sync";

        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(|v: String| {
                        serde_json::from_str(&v).map_err(StoreError::JsonDataDeserializationError)
                    })
            })
            .next()
            .expect("state sync tags exist")
    }

    /// Adds a note tag to the list of tags that the client is interested in.
    pub fn add_note_tag(&mut self, tag: u64) -> Result<bool, StoreError> {
        let mut tags = self.get_note_tags()?;
        if tags.contains(&tag) {
            return Ok(false);
        }
        tags.push(tag);
        let tags = serde_json::to_string(&tags).map_err(StoreError::InputSerializationError)?;

        const QUERY: &str = "UPDATE state_sync SET tags = ?";
        self.db
            .execute(QUERY, params![tags])
            .map_err(StoreError::QueryError)
            .map(|_| ())?;

        Ok(true)
    }

    /// Returns the block number of the last state sync block
    pub fn get_latest_block_num(&self) -> Result<u32, StoreError> {
        const QUERY: &str = "SELECT block_num FROM state_sync";

        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .map(|v: i64| v as u32)
            })
            .next()
            .expect("state sync block number exists")
    }

    pub fn apply_state_sync(
        &mut self,
        block_num: u32,
        nullifiers: Vec<Digest>,
        committed_notes: Vec<(Digest, NoteInclusionProof)>,
    ) -> Result<(), StoreError> {
        let tx = self
            .db
            .transaction()
            .map_err(StoreError::TransactionError)?;

        // update state sync block number
        const BLOCK_NUMBER_QUERY: &str = "UPDATE state_sync SET block_num = ?";
        tx.execute(BLOCK_NUMBER_QUERY, params![block_num])
            .map_err(StoreError::QueryError)?;

        // update spent notes
        for nullifier in nullifiers {
            const SPENT_QUERY: &str =
                "UPDATE input_notes SET status = 'consumed' WHERE nullifier = ?";
            let nullifier = nullifier.to_string();
            tx.execute(SPENT_QUERY, params![nullifier])
                .map_err(StoreError::QueryError)?;
        }

        // update tracked notes
        for (committed_note_hash, inclusion_proof) in committed_notes {
            const SPENT_QUERY: &str =
                "UPDATE input_notes SET status = 'committed', inclusion_proof = ? WHERE hash = ?";
            let inclusion_proof = serde_json::to_string(&inclusion_proof)
                .map_err(StoreError::InputSerializationError)?;
            tx.execute(
                SPENT_QUERY,
                params![inclusion_proof, committed_note_hash.to_string()],
            )
            .map_err(StoreError::QueryError)?;
        }

        // TODO: We would need to mark transactions as committed here as well

        // commit the updates
        tx.commit().map_err(StoreError::QueryError)?;

        Ok(())
    }
}

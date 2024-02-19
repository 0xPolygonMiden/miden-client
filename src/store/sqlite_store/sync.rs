use crypto::{
    merkle::{InOrderIndex, MmrPeaks},
    utils::Serializable,
};

use crate::{errors::StoreError, store::TransactionFilter};
use objects::{
    notes::{NoteId, NoteInclusionProof},
    BlockHeader, Digest,
};
use rusqlite::params;

use super::SqliteStore;

impl SqliteStore {
    pub(crate) fn get_note_tags(&self) -> Result<Vec<u64>, StoreError> {
        const QUERY: &str = "SELECT tags FROM state_sync";

        self.db
            .prepare(QUERY)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(|err| StoreError::ParsingError(err.to_string()))
                    .and_then(|v: String| {
                        serde_json::from_str(&v).map_err(StoreError::JsonDataDeserializationError)
                    })
            })
            .next()
            .expect("state sync tags exist")
    }

    pub(super) fn add_note_tag(&mut self, tag: u64) -> Result<bool, StoreError> {
        let mut tags = self.get_note_tags()?;
        if tags.contains(&tag) {
            return Ok(false);
        }
        tags.push(tag);
        let tags = serde_json::to_string(&tags).map_err(StoreError::InputSerializationError)?;

        const QUERY: &str = "UPDATE state_sync SET tags = ?";
        self.db.execute(QUERY, params![tags])?;

        Ok(true)
    }

    pub(super) fn get_sync_height(&self) -> Result<u32, StoreError> {
        const QUERY: &str = "SELECT block_num FROM state_sync";

        self.db
            .prepare(QUERY)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).map(|v: i64| v as u32))
            .next()
            .expect("state sync block number exists")
    }

    pub(super) fn apply_state_sync(
        &mut self,
        block_header: BlockHeader,
        nullifiers: Vec<Digest>,
        committed_notes: Vec<(NoteId, NoteInclusionProof)>,
        new_mmr_peaks: MmrPeaks,
        new_authentication_nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError> {
        let uncommitted_transactions = self.get_transactions(TransactionFilter::Uncomitted)?;

        let tx = self.db.transaction()?;

        // Update state sync block number
        const BLOCK_NUMBER_QUERY: &str = "UPDATE state_sync SET block_num = ?";
        tx.execute(BLOCK_NUMBER_QUERY, params![block_header.block_num()])?;

        // Update spent notes
        for nullifier in nullifiers {
            const SPENT_INPUT_NOTE_QUERY: &str =
                "UPDATE input_notes SET status = 'consumed' WHERE nullifier = ?";
            let nullifier = nullifier.to_hex();
            tx.execute(SPENT_INPUT_NOTE_QUERY, params![nullifier])?;

            const SPENT_OUTPUT_NOTE_QUERY: &str =
                "UPDATE output_notes SET status = 'consumed' WHERE nullifier = ?";
            tx.execute(SPENT_OUTPUT_NOTE_QUERY, params![nullifier])?;
        }

        // TODO: Due to the fact that notes are returned based on fuzzy matching of tags,
        // this process of marking if the header has notes needs to be revisited
        let block_has_relevant_notes = !committed_notes.is_empty();
        SqliteStore::insert_block_header_tx(
            &tx,
            block_header,
            new_mmr_peaks,
            block_has_relevant_notes,
        )?;

        // Insert new authentication nodes (inner nodes of the PartialMmr)
        SqliteStore::insert_chain_mmr_nodes(&tx, new_authentication_nodes)?;

        // Update tracked notes
        for (note_id, inclusion_proof) in committed_notes.iter() {
            const COMMITTED_INPUT_NOTES_QUERY: &str =
                "UPDATE input_notes SET status = 'committed', inclusion_proof = ? WHERE note_id = ?";

            let inclusion_proof = Some(inclusion_proof.to_bytes());
            tx.execute(
                COMMITTED_INPUT_NOTES_QUERY,
                params![inclusion_proof, note_id.inner().to_hex()],
            )?;

            // Update output notes
            const COMMITTED_OUTPUT_NOTES_QUERY: &str =
                "UPDATE output_notes SET status = 'committed', inclusion_proof = ? WHERE note_id = ?";

            tx.execute(
                COMMITTED_OUTPUT_NOTES_QUERY,
                params![inclusion_proof, note_id.inner().to_hex()],
            )?;
        }

        let note_ids: Vec<NoteId> = committed_notes.iter().map(|(id, _)| (*id)).collect();

        SqliteStore::mark_transactions_as_committed_by_note_id(
            &uncommitted_transactions,
            &note_ids,
            block_header.block_num(),
            &tx,
        )?;

        // Commit the updates
        tx.commit()?;

        Ok(())
    }
}

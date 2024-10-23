use alloc::{collections::BTreeSet, vec::Vec};

use miden_objects::notes::NoteTag;
use miden_tx::utils::{Deserializable, Serializable};
use rusqlite::{params, Transaction};

use super::SqliteStore;
use crate::{
    store::{
        sqlite_store::{
            accounts::update_account,
            notes::{upsert_input_note_tx, upsert_output_note_tx},
        },
        StoreError,
    },
    sync::{NoteTagRecord, NoteTagSource, StateSyncUpdate},
};

impl SqliteStore {
    pub(crate) fn get_note_tags(&self) -> Result<Vec<NoteTagRecord>, StoreError> {
        const QUERY: &str = "SELECT tag, source FROM tags";

        self.db()
            .prepare(QUERY)?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("no binding parameters used in query")
            .map(|result| {
                Ok(result?).and_then(|(tag, source): (Vec<u8>, Vec<u8>)| {
                    Ok(NoteTagRecord {
                        tag: NoteTag::read_from_bytes(&tag)
                            .map_err(StoreError::DataDeserializationError)?,
                        source: NoteTagSource::read_from_bytes(&source)
                            .map_err(StoreError::DataDeserializationError)?,
                    })
                })
            })
            .collect::<Result<Vec<NoteTagRecord>, _>>()
    }

    pub(crate) fn get_unique_note_tags(&self) -> Result<BTreeSet<NoteTag>, StoreError> {
        const QUERY: &str = "SELECT DISTINCT tag FROM tags";

        self.db()
            .prepare(QUERY)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                Ok(result?).and_then(|tag: Vec<u8>| {
                    NoteTag::read_from_bytes(&tag).map_err(StoreError::DataDeserializationError)
                })
            })
            .collect::<Result<BTreeSet<NoteTag>, _>>()
    }

    pub(super) fn add_note_tag(&self, tag: NoteTagRecord) -> Result<bool, StoreError> {
        if self.get_note_tags()?.contains(&tag) {
            return Ok(false);
        }

        let mut db = self.db();
        let tx = db.transaction()?;
        add_note_tag_tx(&tx, &tag)?;

        tx.commit()?;

        Ok(true)
    }

    pub(super) fn remove_note_tag(&self, tag: NoteTagRecord) -> Result<usize, StoreError> {
        let mut db = self.db();
        let tx = db.transaction()?;
        let removed_tags = remove_note_tag_tx(&tx, tag)?;

        tx.commit()?;

        Ok(removed_tags)
    }

    pub(super) fn get_sync_height(&self) -> Result<u32, StoreError> {
        const QUERY: &str = "SELECT block_num FROM state_sync";

        self.db()
            .prepare(QUERY)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).map(|v: i64| v as u32))
            .next()
            .expect("state sync block number exists")
    }

    pub(super) fn apply_state_sync(
        &self,
        state_sync_update: StateSyncUpdate,
    ) -> Result<(), StoreError> {
        let StateSyncUpdate {
            block_header,
            note_updates,
            transactions_to_commit: committed_transactions,
            new_mmr_peaks,
            new_authentication_nodes,
            updated_onchain_accounts,
            block_has_relevant_notes,
            transactions_to_discard: discarded_transactions,
            tags_to_remove,
        } = state_sync_update;

        let mut db = self.db();
        let tx = db.transaction()?;

        // Update state sync block number
        const BLOCK_NUMBER_QUERY: &str = "UPDATE state_sync SET block_num = ?";
        tx.execute(BLOCK_NUMBER_QUERY, params![block_header.block_num()])?;

        Self::insert_block_header_tx(&tx, block_header, new_mmr_peaks, block_has_relevant_notes)?;

        // Upsert notes
        for input_note in
            note_updates.new_public_notes().iter().chain(note_updates.updated_input_notes())
        {
            upsert_input_note_tx(&tx, input_note)?;
        }

        for output_note in note_updates.updated_output_notes() {
            upsert_output_note_tx(&tx, output_note)?;
        }

        // Remove tags
        for tag in tags_to_remove {
            remove_note_tag_tx(&tx, tag)?;
        }

        // Insert new authentication nodes (inner nodes of the PartialMmr)
        Self::insert_chain_mmr_nodes_tx(&tx, &new_authentication_nodes)?;

        // Mark transactions as committed
        Self::mark_transactions_as_committed(&tx, &committed_transactions)?;

        // Marc transactions as discarded
        Self::mark_transactions_as_discarded(&tx, &discarded_transactions)?;

        // Update onchain accounts on the db that have been updated onchain
        for account in updated_onchain_accounts {
            update_account(&tx, &account)?;
        }

        // Commit the updates
        tx.commit()?;

        Ok(())
    }
}

pub(super) fn add_note_tag_tx(tx: &Transaction<'_>, tag: &NoteTagRecord) -> Result<(), StoreError> {
    const QUERY: &str = "INSERT INTO tags (tag, source) VALUES (?, ?)";
    tx.execute(QUERY, params![tag.tag.to_bytes(), tag.source.to_bytes()])?;

    Ok(())
}

pub(super) fn remove_note_tag_tx(
    tx: &Transaction<'_>,
    tag: NoteTagRecord,
) -> Result<usize, StoreError> {
    const QUERY: &str = "DELETE FROM tags WHERE tag = ? AND source = ?";
    let removed_tags = tx.execute(QUERY, params![tag.tag.to_bytes(), tag.source.to_bytes()])?;

    Ok(removed_tags)
}

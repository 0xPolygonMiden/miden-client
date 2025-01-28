use alloc::{collections::BTreeSet, vec::Vec};

use miden_objects::{block::BlockNumber, note::NoteTag};
use miden_tx::utils::{Deserializable, Serializable};
use rusqlite::{params, Connection, Transaction};

use super::SqliteStore;
use crate::{
    store::{
        sqlite_store::{
            account::{lock_account, update_account},
            note::apply_note_updates_tx,
        },
        StoreError,
    },
    sync::{NoteTagRecord, NoteTagSource, StateSyncUpdate},
};

impl SqliteStore {
    pub(crate) fn get_note_tags(conn: &mut Connection) -> Result<Vec<NoteTagRecord>, StoreError> {
        const QUERY: &str = "SELECT tag, source FROM tags";

        conn.prepare(QUERY)?
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

    pub(crate) fn get_unique_note_tags(
        conn: &mut Connection,
    ) -> Result<BTreeSet<NoteTag>, StoreError> {
        const QUERY: &str = "SELECT DISTINCT tag FROM tags";

        conn.prepare(QUERY)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                Ok(result?).and_then(|tag: Vec<u8>| {
                    NoteTag::read_from_bytes(&tag).map_err(StoreError::DataDeserializationError)
                })
            })
            .collect::<Result<BTreeSet<NoteTag>, _>>()
    }

    pub(super) fn add_note_tag(
        conn: &mut Connection,
        tag: NoteTagRecord,
    ) -> Result<bool, StoreError> {
        if Self::get_note_tags(conn)?.contains(&tag) {
            return Ok(false);
        }

        let tx = conn.transaction()?;
        add_note_tag_tx(&tx, &tag)?;

        tx.commit()?;

        Ok(true)
    }

    pub(super) fn remove_note_tag(
        conn: &mut Connection,
        tag: NoteTagRecord,
    ) -> Result<usize, StoreError> {
        let tx = conn.transaction()?;
        let removed_tags = remove_note_tag_tx(&tx, tag)?;

        tx.commit()?;

        Ok(removed_tags)
    }

    pub(super) fn get_sync_height(conn: &mut Connection) -> Result<BlockNumber, StoreError> {
        const QUERY: &str = "SELECT block_num FROM state_sync";

        conn.prepare(QUERY)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).map(|v: i64| BlockNumber::from(v as u32)))
            .next()
            .expect("state sync block number exists")
    }

    pub(super) fn apply_state_sync_step(
        conn: &mut Connection,
        state_sync_update: StateSyncUpdate,
        _block_has_relevant_notes: bool,
    ) -> Result<(), StoreError> {
        let StateSyncUpdate {
            block_headers,
            note_updates,
            transaction_updates,
            account_updates,
            tags_to_remove,
        } = state_sync_update;

        let mut locked_accounts = vec![];

        for (account_id, digest) in account_updates.mismatched_private_accounts() {
            // Mismatched digests may be due to stale network data. If the mismatched digest is
            // tracked in the db and corresponds to the mismatched account, it means we
            // got a past update and shouldn't lock the account.
            if let Some(account) = Self::get_account_header_by_hash(conn, *digest)? {
                if account.id() == *account_id {
                    continue;
                }
            }

            locked_accounts.push(*account_id);
        }

        let tx = conn.transaction()?;

        // Update state sync block number
        const BLOCK_NUMBER_QUERY: &str = "UPDATE state_sync SET block_num = ?";
        if let Some(max_block_num) =
            block_headers.iter().map(|(header, ..)| header.block_num().as_u32()).max()
        {
            tx.execute(BLOCK_NUMBER_QUERY, params![max_block_num as i64])?;
        }

        for (block_header, block_has_relevant_notes, new_mmr_peaks, new_authentication_nodes) in
            block_headers
        {
            Self::insert_block_header_tx(
                &tx,
                block_header,
                new_mmr_peaks,
                block_has_relevant_notes,
            )?;

            // Insert new authentication nodes (inner nodes of the PartialMmr)
            Self::insert_chain_mmr_nodes_tx(&tx, &new_authentication_nodes)?;
        }
        // Update notes
        apply_note_updates_tx(&tx, &note_updates)?;

        // Remove tags
        for tag in tags_to_remove {
            remove_note_tag_tx(&tx, tag)?;
        }

        // Mark transactions as committed
        Self::mark_transactions_as_committed(&tx, transaction_updates.committed_transactions())?;

        // Marc transactions as discarded
        Self::mark_transactions_as_discarded(&tx, transaction_updates.discarded_transactions())?;

        // Update public accounts on the db that have been updated onchain
        for account in account_updates.updated_public_accounts() {
            update_account(&tx, account)?;
        }

        for account_id in locked_accounts {
            lock_account(&tx, account_id)?;
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

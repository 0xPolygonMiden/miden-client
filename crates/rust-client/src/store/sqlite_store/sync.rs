#![allow(clippy::items_after_statements)]

use alloc::{collections::BTreeSet, vec::Vec};

use miden_objects::{Digest, block::BlockNumber, note::NoteTag, transaction::TransactionId};
use miden_tx::utils::{Deserializable, Serializable};
use rusqlite::{Connection, Transaction, params};

use super::{SqliteStore, account::undo_account_state};
use crate::{
    insert_sql,
    note::NoteUpdates,
    store::{
        StoreError, TransactionFilter,
        sqlite_store::{
            account::{lock_account, update_account},
            note::apply_note_updates_tx,
        },
    },
    subst,
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
            .map(|result| {
                Ok(result?).map(|v: i64| {
                    BlockNumber::from(u32::try_from(v).expect("block number is always positive"))
                })
            })
            .next()
            .expect("state sync block number exists")
    }

    pub(super) fn apply_state_sync(
        conn: &mut Connection,
        state_sync_update: StateSyncUpdate,
    ) -> Result<(), StoreError> {
        let StateSyncUpdate {
            block_header,
            block_has_relevant_notes,
            new_mmr_peaks,
            new_authentication_nodes,
            note_updates,
            transaction_updates,
            account_updates,
            tags_to_remove,
        } = state_sync_update;

        let tx = conn.transaction()?;

        // Update state sync block number
        const BLOCK_NUMBER_QUERY: &str = "UPDATE state_sync SET block_num = ?";
        tx.execute(BLOCK_NUMBER_QUERY, params![i64::from(block_header.block_num().as_u32())])?;

        Self::insert_block_header_tx(&tx, &block_header, &new_mmr_peaks, block_has_relevant_notes)?;

        // Update notes
        apply_note_updates_tx(&tx, &note_updates)?;

        // Remove tags
        for tag in tags_to_remove {
            remove_note_tag_tx(&tx, tag)?;
        }

        // Insert new authentication nodes (inner nodes of the PartialMmr)
        Self::insert_partial_blockchain_nodes_tx(&tx, &new_authentication_nodes)?;

        // Mark transactions as committed
        Self::mark_transactions_as_committed(&tx, transaction_updates.committed_transactions())?;

        // Delete accounts for old pending transactions
        let account_hashes_to_delete: Vec<Digest> = transaction_updates
            .stale_transactions()
            .iter()
            .map(|tx| tx.final_account_state)
            .collect();

        undo_account_state(&tx, &account_hashes_to_delete)?;

        // Combine discarded transactions from sync and old pending transactions
        let mut discarded_transactions = transaction_updates.discarded_transactions().to_vec();
        discarded_transactions
            .extend(transaction_updates.stale_transactions().iter().map(|tx| tx.id));

        // Mark all transactions as discarded in a single call
        Self::mark_transactions_as_discarded(&tx, &discarded_transactions)?;

        // Update public accounts on the db that have been updated onchain
        for account in account_updates.updated_public_accounts() {
            update_account(&tx, account)?;
        }

        for (account_id, _) in account_updates.mismatched_private_accounts() {
            lock_account(&tx, *account_id)?;
        }

        // Commit the updates
        tx.commit()?;

        Ok(())
    }

    pub(super) fn apply_nullifiers(
        conn: &mut Connection,
        note_updates: &NoteUpdates,
        transactions_to_discard: &[TransactionId],
    ) -> Result<(), StoreError> {
        // First we need the `transaction` entries from the `transactions` table that matches the
        // `transactions_to_discard`

        let transactions_records_to_discard = Self::get_transactions(
            conn,
            &TransactionFilter::Ids(transactions_to_discard.to_vec()),
        )?;

        let tx = conn.transaction()?;

        apply_note_updates_tx(&tx, note_updates)?;

        Self::mark_transactions_as_discarded(&tx, transactions_to_discard)?;

        let final_account_states = transactions_records_to_discard
            .iter()
            .map(|tx_record| tx_record.final_account_state)
            .collect::<Vec<_>>();

        // Remove the accounts that are originated from the discarded transactions
        undo_account_state(&tx, &final_account_states)?;

        tx.commit()?;

        Ok(())
    }
}

pub(super) fn add_note_tag_tx(tx: &Transaction<'_>, tag: &NoteTagRecord) -> Result<(), StoreError> {
    const QUERY: &str = insert_sql!(tags { tag, source });
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

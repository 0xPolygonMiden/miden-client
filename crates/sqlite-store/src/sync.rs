#![allow(clippy::items_after_statements)]

use std::collections::BTreeSet;
use std::vec::Vec;

use miden_client::Word;
use miden_client::account::AccountId;
use miden_client::note::{BlockNumber, NoteTag};
use miden_client::protocol_config::protocol_config_setting_key;
use miden_client::store::{SettingScope, StoreError};
use miden_client::sync::{NoteTagRecord, NoteTagSource, PublicAccountUpdate, StateSyncUpdate};
use miden_client::utils::{Deserializable, Serializable};
use rusqlite::{Connection, Transaction, params};

use super::SqliteStore;
use crate::chain_data::{TrackedPaths, check_tracked_paths_tx};
use crate::forest::{ScopedAccountForest, SqliteForestBackend};
use crate::note::apply_note_updates_tx;
use crate::sql_error::SqlResultExt;
use crate::transaction::upsert_transaction_record;
use crate::{insert_sql, proto, subst, with_write_tx};

impl SqliteStore {
    pub(crate) fn get_note_tags(conn: &mut Connection) -> Result<Vec<NoteTagRecord>, StoreError> {
        const QUERY: &str = "SELECT tag, source FROM tags";

        conn.prepare_cached(QUERY)
            .into_store_error()?
            .query_map([], |row| Ok((row.get("tag")?, row.get("source")?)))
            .expect("no binding parameters used in query")
            .map(|result| {
                let (tag, source): (Vec<u8>, Vec<u8>) = result.into_store_error()?;
                Ok(NoteTagRecord {
                    tag: NoteTag::read_from_bytes(&tag)
                        .map_err(StoreError::DataDeserializationError)?,
                    source: NoteTagSource::read_from_bytes(&source)
                        .map_err(StoreError::DataDeserializationError)?,
                })
            })
            .collect::<Result<Vec<NoteTagRecord>, _>>()
    }

    pub(crate) fn get_unique_note_tags(
        conn: &mut Connection,
    ) -> Result<BTreeSet<NoteTag>, StoreError> {
        const QUERY: &str = "SELECT DISTINCT tag FROM tags";

        conn.prepare_cached(QUERY)
            .into_store_error()?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                let tag: Vec<u8> = result.into_store_error()?;
                NoteTag::read_from_bytes(&tag).map_err(StoreError::DataDeserializationError)
            })
            .collect::<Result<BTreeSet<NoteTag>, _>>()
    }

    pub(super) fn add_note_tag(
        conn: &mut Connection,
        tag: NoteTagRecord,
    ) -> Result<bool, StoreError> {
        with_write_tx(conn, |tx| add_note_tag_tx(tx, &tag))
    }

    pub(super) fn remove_note_tag(
        conn: &mut Connection,
        tag: NoteTagRecord,
    ) -> Result<usize, StoreError> {
        with_write_tx(conn, |tx| remove_note_tag_tx(tx, tag))
    }

    pub(super) fn get_sync_height(conn: &mut Connection) -> Result<BlockNumber, StoreError> {
        query_sync_height(conn)
    }

    pub(super) fn apply_state_sync(
        conn: &mut Connection,
        state_sync_update: StateSyncUpdate,
    ) -> Result<(), StoreError> {
        let block_from = state_sync_update.block_from();
        let (
            block_num,
            partial_blockchain_updates,
            note_updates,
            transaction_updates,
            account_updates,
            protocol_config,
        ) = state_sync_update.into_parts();

        with_write_tx(conn, |db_tx| {
            // The update was computed from the chain data at `block_from`. If another writer moved
            // the sync height since then, the update is stale.
            if let Some(block_from) = block_from {
                let sync_height = query_sync_height(db_tx)?;
                if sync_height != block_from {
                    return Err(StoreError::DatabaseTransientError(format!(
                        "stale chain write: the sync update starts at block {block_from} but the \
                         sync height is {sync_height}"
                    )));
                }
            }

            let mut smt_forest = ScopedAccountForest::new(SqliteForestBackend::new(db_tx))?;
            // Update blockchain checkpoint (block number and peaks) only if moving forward.
            let new_peaks_bytes = proto::encode_mmr_peaks(&partial_blockchain_updates.new_peaks);
            const BLOCKCHAIN_CHECKPOINT_QUERY: &str = "\
                UPDATE blockchain_checkpoint \
                SET block_num = ?1, partial_blockchain_peaks = ?2 \
                WHERE block_num < ?1";
            let checkpoint_moved = db_tx
                .execute(BLOCKCHAIN_CHECKPOINT_QUERY, params![block_num.as_u32(), new_peaks_bytes])
                .into_store_error()?
                > 0;

            let mut tracks_block = false;
            for (block_header, is_relevant) in
                partial_blockchain_updates.block_headers_to_store(block_num)
            {
                Self::insert_block_header_tx(db_tx, block_header, *is_relevant)?;
                tracks_block |= *is_relevant;
            }

            // Insert new authentication nodes (inner nodes of the PartialBlockchain)
            Self::insert_partial_blockchain_nodes_tx(
                db_tx,
                partial_blockchain_updates.new_authentication_nodes(),
            )?;

            // A tree merge at the new sync height adds levels to the paths of tracked blocks. The
            // update has the new nodes only for the blocks in the view that it was computed from.
            if checkpoint_moved || tracks_block {
                check_tracked_paths_tx(db_tx, TrackedPaths::All)?;
            }

            // Update notes
            apply_note_updates_tx(db_tx, &note_updates)?;

            // Remove tags of input notes whose inclusion settled in this sync (committed, consumed
            // during catch-up, or invalidated): their tag no longer drives note sync. Metadata-less
            // records are skipped; their tag (if any) cannot be reconstructed.
            let tags_to_remove = note_updates
                .updated_input_notes()
                .filter_map(|note_update| {
                    let note = note_update.inner();
                    if note.is_inclusion_pending() {
                        None
                    } else {
                        Some(NoteTagRecord {
                            tag: note.metadata()?.tag(),
                            source: NoteTagSource::Note(note.details_commitment()),
                        })
                    }
                })
                .collect::<Vec<_>>();

            for tag in tags_to_remove {
                remove_note_tag_tx(db_tx, tag)?;
            }

            for transaction_record in transaction_updates
                .committed_transactions()
                .chain(transaction_updates.discarded_transactions())
            {
                upsert_transaction_record(db_tx, transaction_record)?;
            }

            // Remove the accounts that are originated from the discarded transactions
            let discarded_states: Vec<(AccountId, Word)> = transaction_updates
                .discarded_transactions()
                .map(|tx| (tx.details.account_id, tx.details.final_account_state))
                .collect();

            Self::undo_account_state(db_tx, &mut smt_forest, &discarded_states)?;

            // Update public accounts on the db that have been updated onchain
            for update in account_updates.updated_public_accounts() {
                match update {
                    PublicAccountUpdate::Full(account) => {
                        Self::update_account_state(db_tx, &mut smt_forest, account)?;
                    },
                    PublicAccountUpdate::Patch { new_header, patch } => {
                        Self::apply_sync_account_patch(db_tx, &mut smt_forest, new_header, patch)?;
                    },
                }
            }

            for (account_id, digest) in account_updates.mismatched_private_accounts() {
                Self::lock_account_on_unexpected_commitment(db_tx, account_id, digest)?;
            }

            // Writes only land for accounts registered for witness prefetching; the rest are
            // no-ops, so the sync does not have to know which ones those are.
            for (account_id, witness) in account_updates.account_witnesses() {
                Self::update_account_witness_tx(db_tx, *account_id, witness)?;
            }

            if let Some(config) = &protocol_config {
                Self::set_setting(
                    db_tx,
                    SettingScope::Client,
                    &protocol_config_setting_key(config.to_commitment()),
                    &config.to_bytes(),
                )?;
            }

            Ok(())
        })
    }
}

/// Inserts the tag record, relying on the `(tag, source)` primary key for idempotency across
/// concurrent connections. Returns whether a new row was inserted.
pub(super) fn add_note_tag_tx(
    tx: &Transaction<'_>,
    tag: &NoteTagRecord,
) -> Result<bool, StoreError> {
    const QUERY: &str = insert_sql!(tags { tag, source } | IGNORE);
    let inserted = tx
        .execute(QUERY, params![tag.tag.to_bytes(), tag.source.to_bytes()])
        .into_store_error()?;

    Ok(inserted > 0)
}

pub(super) fn remove_note_tag_tx(
    tx: &Transaction<'_>,
    tag: NoteTagRecord,
) -> Result<usize, StoreError> {
    const QUERY: &str = "DELETE FROM tags WHERE tag = ? AND source = ?";
    let removed_tags = tx
        .execute(QUERY, params![tag.tag.to_bytes(), tag.source.to_bytes()])
        .into_store_error()?;

    Ok(removed_tags)
}

/// Reads the sync height from the `blockchain_checkpoint` row.
///
/// The initial migration seeds the row, so a missing row is corruption and returns a
/// [`StoreError::QueryError`].
pub(crate) fn query_sync_height(conn: &Connection) -> Result<BlockNumber, StoreError> {
    conn.query_row("SELECT block_num FROM blockchain_checkpoint LIMIT 1", [], |row| {
        row.get::<_, u32>("block_num")
    })
    .into_store_error()
    .map(BlockNumber::from)
}

#[cfg(test)]
mod tests {
    use miden_client::crypto::{Forest, PartialMmr};
    use miden_client::note::{BlockNumber, NoteUpdateTracker};
    use miden_client::store::{Store, StoreError};
    use miden_client::sync::{
        AccountUpdates,
        PartialBlockchainUpdates,
        StateSyncUpdate,
        TransactionUpdateTracker,
    };
    use rusqlite::Connection;

    use crate::SqliteStore;
    use crate::chain_data::test::{mock_chain, path_nodes, set_sync_height};
    use crate::db_management::migration::SqliteMigrator;
    use crate::tests::create_test_store;

    /// Tests that the store refuses a sync update after which a tracked block lacks the siblings
    /// that a tree merge in the update adds to its path.
    #[tokio::test]
    async fn apply_state_sync_rejects_update_that_drops_tracked_siblings() {
        let store = create_test_store().await;
        let (headers, mmr) = mock_chain(42);
        set_sync_height(&store, &mmr, &headers, 1).await;

        // At forest 2, leaf 0 has 1 sibling. At forest 42 it is in the tree of 32 leaves and has 5.
        let tracked_block = 0;
        Store::insert_block_header(
            &store,
            &headers[tracked_block],
            &path_nodes(&mmr, &headers, tracked_block, Forest::new(2).unwrap()),
            true,
        )
        .await
        .unwrap();

        // The update comes from a view that does not track the block, so it has none of the new
        // siblings.
        let new_height = 41;
        let mut view = PartialMmr::from_peaks(mmr.peaks_at(Forest::new(1).unwrap()).unwrap());
        view.add(headers[1].commitment(), false).unwrap();
        let mut new_nodes = view
            .apply(
                mmr.get_delta(Forest::new(2).unwrap(), Forest::new(new_height).unwrap())
                    .unwrap(),
            )
            .unwrap();
        let mut partial_blockchain_updates = PartialBlockchainUpdates::default();
        partial_blockchain_updates.new_peaks = view.peaks();
        new_nodes.extend(view.add(headers[new_height].commitment(), false).unwrap());
        partial_blockchain_updates.insert(headers[new_height].clone(), false);
        partial_blockchain_updates.extend_authentication_nodes(new_nodes);

        let update = StateSyncUpdate::from_parts(
            BlockNumber::from(u32::try_from(new_height).unwrap()),
            partial_blockchain_updates,
            NoteUpdateTracker::default(),
            TransactionUpdateTracker::default(),
            AccountUpdates::default(),
            None,
        );
        let result = store.apply_state_sync(update).await;
        assert!(
            matches!(result, Err(StoreError::DatabaseTransientError(_))),
            "an update that leaves a tracked path short must be refused, got {result:?}"
        );

        assert_eq!(store.get_sync_height().await.unwrap(), BlockNumber::from(1u32));
        let partial_mmr = store.get_current_partial_mmr().await.unwrap();
        assert!(partial_mmr.open(tracked_block).unwrap().is_some());
    }

    /// A missing checkpoint row is only reachable through corruption (the initial migration seeds
    /// it); it must surface as an error, not a panic.
    #[test]
    fn get_sync_height_errors_when_checkpoint_is_missing() {
        let mut conn = Connection::open_in_memory().unwrap();
        SqliteMigrator::client().apply(&mut conn).unwrap();
        conn.execute("DELETE FROM blockchain_checkpoint", []).unwrap();

        let err = SqliteStore::get_sync_height(&mut conn).unwrap_err();
        assert!(matches!(err, miden_client::store::StoreError::QueryError(_)));
    }
}

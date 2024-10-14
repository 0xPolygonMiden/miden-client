use alloc::{collections::BTreeSet, string::ToString, vec::Vec};

use miden_objects::notes::{NoteTag, Nullifier};
use miden_tx::utils::{Deserializable, Serializable};
use rusqlite::{named_params, params, Transaction};

use super::SqliteStore;
use crate::{
    store::{
        sqlite_store::{
            accounts::update_account,
            notes::{upsert_input_note_tx, upsert_output_note_tx},
        },
        CommittedNoteState, InputNoteRecord, NoteFilter, OutputNoteRecord, StoreError,
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
        add_note_tag_tx(&tx, tag)?;

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
        let (mut relevant_input_notes, mut relevant_output_notes) =
            self.get_relevant_sync_notes(&state_sync_update)?;

        let StateSyncUpdate {
            block_header,
            nullifiers,
            synced_new_notes: committed_notes,
            transactions_to_commit: committed_transactions,
            new_mmr_peaks,
            new_authentication_nodes,
            updated_onchain_accounts,
            block_has_relevant_notes,
        } = state_sync_update;

        let mut db = self.db();
        let tx = db.transaction()?;

        // Update state sync block number
        const BLOCK_NUMBER_QUERY: &str = "UPDATE state_sync SET block_num = ?";
        tx.execute(BLOCK_NUMBER_QUERY, params![block_header.block_num()])?;

        // Update tracked output notes
        for (note_id, inclusion_proof) in committed_notes.updated_output_notes().iter() {
            if let Some(output_note_record) =
                relevant_output_notes.iter_mut().find(|n| n.id() == *note_id)
            {
                if output_note_record.inclusion_proof_received(inclusion_proof.clone())? {
                    upsert_output_note_tx(&tx, output_note_record)?;
                }
            }
        }

        // Update tracked input notes
        for input_note in committed_notes.updated_input_notes().iter() {
            let inclusion_proof = input_note.proof().ok_or(StoreError::DatabaseError(
                "Input note doesn't have inclusion proof".to_string(),
            ))?;
            let metadata = input_note.note().metadata();

            if let Some(input_note_record) =
                relevant_input_notes.iter_mut().find(|n| n.id() == input_note.id())
            {
                let inclusion_proof_received = input_note_record
                    .inclusion_proof_received(inclusion_proof.clone(), *metadata)?;
                let block_header_received =
                    input_note_record.block_header_received(block_header)?;

                if inclusion_proof_received || block_header_received {
                    upsert_input_note_tx(&tx, input_note_record)?;
                }

                remove_note_tag_tx(
                    &tx,
                    NoteTagRecord::with_note_source(
                        input_note.note().metadata().tag(),
                        input_note_record.id(),
                    ),
                )?;
            }
        }

        // Commit new public notes
        for input_note in committed_notes.new_public_notes() {
            let details = input_note.note().into();

            let input_note_record = InputNoteRecord::new(
                details,
                None,
                CommittedNoteState {
                    metadata: *input_note.note().metadata(),
                    inclusion_proof: input_note
                        .proof()
                        .expect("New public note should be authenticated")
                        .clone(),
                    block_note_root: block_header.note_root(),
                }
                .into(),
            );

            upsert_input_note_tx(&tx, &input_note_record)?;
        }

        // Committed transactions
        for transaction_update in committed_transactions.iter() {
            if let Some(input_note_record) = relevant_input_notes.iter_mut().find(|n| {
                n.is_processing()
                    && n.consumer_transaction_id() == Some(&transaction_update.transaction_id)
            }) {
                if input_note_record.transaction_committed(
                    transaction_update.transaction_id,
                    transaction_update.block_num,
                )? {
                    upsert_input_note_tx(&tx, input_note_record)?;
                }
            }
        }

        // Update spent notes
        let mut discarded_transactions = vec![];
        for nullifier_update in nullifiers.iter() {
            let nullifier = nullifier_update.nullifier;
            let block_num = nullifier_update.block_num;

            if let Some(input_note_record) =
                relevant_input_notes.iter_mut().find(|n| n.nullifier() == nullifier)
            {
                if input_note_record.is_processing() {
                    discarded_transactions.push(
                        *input_note_record
                            .consumer_transaction_id()
                            .expect("Processing note should have consumer transaction id"),
                    );
                }

                if input_note_record.consumed_externally(nullifier, block_num)? {
                    upsert_input_note_tx(&tx, input_note_record)?;
                }
            }

            if let Some(output_note_record) = relevant_output_notes
                .iter_mut()
                .find(|n| n.nullifier().is_some_and(|n| n == nullifier))
            {
                if output_note_record.nullifier_received(nullifier, block_num)? {
                    upsert_output_note_tx(&tx, output_note_record)?;
                }
            }
        }

        Self::insert_block_header_tx(&tx, block_header, new_mmr_peaks, block_has_relevant_notes)?;

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

    /// Get the input notes from the store that are relevant to the state sync update. Secifically,
    /// notes that were updated and nullified during the sync.
    fn get_relevant_sync_notes(
        &self,
        state_sync_update: &StateSyncUpdate,
    ) -> Result<(Vec<InputNoteRecord>, Vec<OutputNoteRecord>), StoreError> {
        let StateSyncUpdate { nullifiers, synced_new_notes, .. } = state_sync_update;

        let updated_input_note_ids = synced_new_notes
            .updated_input_notes()
            .iter()
            .map(|input_note| input_note.id())
            .collect::<Vec<_>>();

        let updated_output_note_ids = synced_new_notes
            .updated_output_notes()
            .iter()
            .map(|output_note| output_note.0)
            .collect::<Vec<_>>();

        let updated_input_notes = self.get_input_notes(NoteFilter::List(updated_input_note_ids))?;
        let updated_output_notes =
            self.get_output_notes(NoteFilter::List(updated_output_note_ids))?;

        let nullifiers: Vec<Nullifier> = nullifiers
            .iter()
            .map(|nullifier_update| nullifier_update.nullifier)
            .collect::<Vec<_>>();

        let nullified_input_notes =
            self.get_input_notes(NoteFilter::Nullifiers(nullifiers.clone()))?;
        let nullified_output_notes = self.get_output_notes(NoteFilter::Nullifiers(nullifiers))?;

        let mut relevant_input_notes = updated_input_notes;
        relevant_input_notes.extend(nullified_input_notes);

        let mut relevant_output_notes = updated_output_notes;
        relevant_output_notes.extend(nullified_output_notes);

        Ok((relevant_input_notes, relevant_output_notes))
    }
}

pub(super) fn add_note_tag_tx(tx: &Transaction<'_>, tag: NoteTagRecord) -> Result<(), StoreError> {
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

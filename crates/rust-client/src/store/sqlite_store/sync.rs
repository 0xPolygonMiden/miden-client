use alloc::{string::ToString, vec::Vec};

use miden_objects::notes::{NoteInclusionProof, NoteTag};
use miden_tx::utils::{Deserializable, Serializable};
use rusqlite::{named_params, params};

use super::SqliteStore;
use crate::{
    store::{
        note_record::{NOTE_STATUS_COMMITTED, NOTE_STATUS_CONSUMED},
        sqlite_store::{accounts::update_account, notes::insert_input_note_tx},
        StoreError,
    },
    sync::StateSyncUpdate,
};

impl SqliteStore {
    pub(crate) fn get_note_tags(&self) -> Result<Vec<NoteTag>, StoreError> {
        const QUERY: &str = "SELECT tags FROM state_sync";

        self.db()
            .prepare(QUERY)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                result.map_err(|err| StoreError::ParsingError(err.to_string())).and_then(
                    |v: Option<Vec<u8>>| match v {
                        Some(tags) => Vec::<NoteTag>::read_from_bytes(&tags)
                            .map_err(StoreError::DataDeserializationError),
                        None => Ok(Vec::<NoteTag>::new()),
                    },
                )
            })
            .next()
            .expect("state sync tags exist")
    }

    pub(super) fn add_note_tag(&self, tag: NoteTag) -> Result<bool, StoreError> {
        let mut tags = self.get_note_tags()?;
        if tags.contains(&tag) {
            return Ok(false);
        }
        tags.push(tag);
        let tags = tags.to_bytes();

        const QUERY: &str = "UPDATE state_sync SET tags = :tags";
        self.db().execute(QUERY, named_params! {":tags": tags})?;

        const IGNORED_NOTES_QUERY: &str =
            "UPDATE input_notes SET ignored = 0 WHERE imported_tag = ?";
        self.db().execute(IGNORED_NOTES_QUERY, params![u32::from(tag)])?;

        Ok(true)
    }

    pub(super) fn remove_note_tag(&self, tag: NoteTag) -> Result<bool, StoreError> {
        let mut tags = self.get_note_tags()?;
        if let Some(index_of_tag) = tags.iter().position(|&tag_candidate| tag_candidate == tag) {
            tags.remove(index_of_tag);

            let tags = tags.to_bytes();

            const QUERY: &str = "UPDATE state_sync SET tags = ?";
            self.db().execute(QUERY, params![tags])?;
            return Ok(true);
        }

        Ok(false)
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
            let block_num = inclusion_proof.location().block_num();
            let note_index = inclusion_proof.location().node_index_in_block();

            let inclusion_proof = NoteInclusionProof::new(
                block_num,
                note_index,
                inclusion_proof.note_path().clone(),
            )?
            .to_bytes();

            // Update output notes
            const COMMITTED_OUTPUT_NOTES_QUERY: &str =
                "UPDATE output_notes SET status = :status , inclusion_proof = :inclusion_proof WHERE note_id = :note_id";

            tx.execute(
                COMMITTED_OUTPUT_NOTES_QUERY,
                named_params! {
                    ":inclusion_proof": inclusion_proof,
                    ":note_id": note_id.inner().to_hex(),
                    ":status": NOTE_STATUS_COMMITTED.to_string(),
                },
            )?;
        }

        // Update tracked input notes
        for input_note in committed_notes.updated_input_notes().iter() {
            let inclusion_proof = input_note.proof().ok_or(StoreError::DatabaseError(
                "Input note doesn't have inclusion proof".to_string(),
            ))?;
            let metadata = input_note.note().metadata();

            let inclusion_proof = inclusion_proof.to_bytes();
            let metadata = metadata.to_bytes();

            const COMMITTED_INPUT_NOTES_QUERY: &str =
                "UPDATE input_notes SET status = :status , inclusion_proof = :inclusion_proof, metadata = :metadata WHERE note_id = :note_id";

            tx.execute(
                COMMITTED_INPUT_NOTES_QUERY,
                named_params! {
                    ":inclusion_proof": inclusion_proof,
                    ":metadata": metadata,
                    ":note_id": input_note.id().inner().to_hex(),
                    ":status": NOTE_STATUS_COMMITTED.to_string(),
                },
            )?;
        }

        // Commit new public notes
        for note in committed_notes.new_public_notes() {
            insert_input_note_tx(
                &tx,
                note.location().expect("new public note should be authenticated").block_num(),
                note.clone().into(),
            )?;
        }

        // Update spent notes
        for nullifier_update in nullifiers.iter() {
            const SPENT_INPUT_NOTE_QUERY: &str =
                "UPDATE input_notes SET status = ?, nullifier_height = ? WHERE nullifier = ?";
            let nullifier = nullifier_update.nullifier.to_hex();
            let block_num = nullifier_update.block_num;
            tx.execute(
                SPENT_INPUT_NOTE_QUERY,
                params![NOTE_STATUS_CONSUMED.to_string(), block_num, nullifier],
            )?;

            const SPENT_OUTPUT_NOTE_QUERY: &str =
                "UPDATE output_notes SET status = ?, nullifier_height = ? WHERE nullifier = ?";
            tx.execute(
                SPENT_OUTPUT_NOTE_QUERY,
                params![NOTE_STATUS_CONSUMED.to_string(), block_num, nullifier],
            )?;
        }

        Self::insert_block_header_tx(&tx, block_header, new_mmr_peaks, block_has_relevant_notes)?;

        // Insert new authentication nodes (inner nodes of the PartialMmr)
        Self::insert_chain_mmr_nodes_tx(&tx, &new_authentication_nodes)?;

        // Mark transactions as committed
        Self::mark_transactions_as_committed(&tx, &committed_transactions)?;

        // Update onchain accounts on the db that have been updated onchain
        for account in updated_onchain_accounts {
            update_account(&tx, &account)?;
        }

        // Commit the updates
        tx.commit()?;

        Ok(())
    }
}

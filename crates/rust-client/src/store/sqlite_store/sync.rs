use alloc::{string::ToString, vec::Vec};

use miden_objects::notes::{NoteInclusionProof, NoteTag};
use miden_tx::utils::{Deserializable, Serializable};
use rusqlite::{named_params, params};

use super::SqliteStore;
use crate::{
    store::{
        note_record::{NOTE_STATUS_COMMITTED, NOTE_STATUS_CONSUMED},
        sqlite_store::{accounts::update_account, notes::upsert_input_note_tx},
        CommittedNoteState, InputNoteRecord, NoteFilter, StoreError,
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
        let mut relevant_input_notes = self.get_relevant_sync_input_notes(&state_sync_update)?;

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

            let note_position = relevant_input_notes.iter().position(|n| n.id() == input_note.id());

            if let Some(note_position) = note_position {
                let mut input_note_record = relevant_input_notes.swap_remove(note_position);

                let inclusion_proof_received = input_note_record
                    .inclusion_proof_received(inclusion_proof.clone(), *metadata)?;
                let block_header_received =
                    input_note_record.block_header_received(block_header)?;

                if inclusion_proof_received || block_header_received {
                    upsert_input_note_tx(&tx, input_note_record)?;
                }
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

            upsert_input_note_tx(&tx, input_note_record)?;
        }

        // Update spent notes
        for nullifier_update in nullifiers.iter() {
            let nullifier = nullifier_update.nullifier;
            let block_num = nullifier_update.block_num;

            let note_pos = relevant_input_notes.iter().position(|n| n.nullifier() == nullifier);

            if let Some(note_pos) = note_pos {
                let mut input_note_record = relevant_input_notes.swap_remove(note_pos);

                if input_note_record.nullifier_received(nullifier, block_num)? {
                    upsert_input_note_tx(&tx, input_note_record)?;
                }
            }

            const SPENT_OUTPUT_NOTE_QUERY: &str =
                "UPDATE output_notes SET status = ?, nullifier_height = ? WHERE nullifier = ?";
            tx.execute(
                SPENT_OUTPUT_NOTE_QUERY,
                params![NOTE_STATUS_CONSUMED.to_string(), block_num, nullifier.to_hex()],
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

    /// Get the input notes from the store that are relevant to the state sync update. Secifically,
    /// notes that were updated and nullified during the sync.
    fn get_relevant_sync_input_notes(
        &self,
        state_sync_update: &StateSyncUpdate,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        let StateSyncUpdate { nullifiers, synced_new_notes, .. } = state_sync_update;

        let updated_input_note_ids = synced_new_notes
            .updated_input_notes()
            .iter()
            .map(|input_note| input_note.id())
            .collect::<Vec<_>>();
        let updated_input_notes =
            self.get_input_notes(NoteFilter::List(&updated_input_note_ids))?;

        let nullifiers = nullifiers
            .iter()
            .map(|nullifier_update| nullifier_update.nullifier)
            .collect::<Vec<_>>();
        let nullified_notes = self.get_input_notes(NoteFilter::Nullifiers(&nullifiers))?;

        let mut relevant_notes = updated_input_notes;
        relevant_notes.extend(nullified_notes);

        Ok(relevant_notes)
    }
}

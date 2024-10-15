use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    accounts::AccountId,
    notes::{NoteId, NoteInclusionProof, NoteTag},
};
use miden_tx::utils::{Deserializable, Serializable};
use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::*;

use super::{
    chain_data::utils::serialize_chain_mmr_node, notes::utils::upsert_input_note_tx,
    transactions::utils::update_account, WebStore,
};
use crate::{
    store::{CommittedNoteState, InputNoteRecord, NoteFilter, StoreError},
    sync::{NoteTagRecord, NoteTagSource, StateSyncUpdate},
};

mod js_bindings;
use js_bindings::*;

mod models;
use models::*;

mod flattened_vec;
use flattened_vec::*;

impl WebStore {
    pub(crate) async fn get_note_tags(&self) -> Result<Vec<NoteTagRecord>, StoreError> {
        let promise = idxdb_get_note_tags();
        let js_value = JsFuture::from(promise).await.unwrap();
        let tags_idxdb: Vec<NoteTagIdxdbObject> = from_value(js_value).unwrap();

        let tags = tags_idxdb
            .into_iter()
            .map(|t| -> Result<NoteTagRecord, StoreError> {
                let source = match (t.source_account_id, t.source_note_id) {
                    (None, None) => NoteTagSource::User,
                    (Some(account_id), None) => {
                        NoteTagSource::Account(AccountId::from_hex(account_id.as_str())?)
                    },
                    (None, Some(note_id)) => {
                        NoteTagSource::Note(NoteId::try_from_hex(note_id.as_str())?)
                    },
                    _ => return Err(StoreError::ParsingError("Invalid NoteTagSource".to_string())),
                };

                Ok(NoteTagRecord {
                    tag: NoteTag::read_from_bytes(&t.tag)?,
                    source,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tags)
    }

    pub(super) async fn get_sync_height(&self) -> Result<u32, StoreError> {
        let promise = idxdb_get_sync_height();
        let js_value = JsFuture::from(promise).await.unwrap();
        let block_num_idxdb: SyncHeightIdxdbObject = from_value(js_value).unwrap();

        let block_num_as_u32: u32 = block_num_idxdb.block_num.parse::<u32>().unwrap();
        Ok(block_num_as_u32)
    }

    pub(super) async fn add_note_tag(&self, tag: NoteTagRecord) -> Result<bool, StoreError> {
        if self.get_note_tags().await?.contains(&tag) {
            return Ok(false);
        }

        let (source_note_id, source_account_id) = match tag.source {
            NoteTagSource::Note(note_id) => (Some(note_id.to_hex()), None),
            NoteTagSource::Account(account_id) => (None, Some(account_id.to_hex())),
            NoteTagSource::User => (None, None),
        };

        let promise = idxdb_add_note_tag(tag.tag.to_bytes(), source_note_id, source_account_id);
        JsFuture::from(promise).await.unwrap();

        Ok(true)
    }

    pub(super) async fn remove_note_tag(&self, tag: NoteTagRecord) -> Result<usize, StoreError> {
        let (source_note_id, source_account_id) = match tag.source {
            NoteTagSource::Note(note_id) => (Some(note_id.to_hex()), None),
            NoteTagSource::Account(account_id) => (None, Some(account_id.to_hex())),
            NoteTagSource::User => (None, None),
        };

        let promise = idxdb_remove_note_tag(tag.tag.to_bytes(), source_note_id, source_account_id);
        let removed_tags = from_value(JsFuture::from(promise).await.unwrap()).unwrap();

        Ok(removed_tags)
    }

    pub(super) async fn apply_state_sync(
        &self,
        state_sync_update: StateSyncUpdate,
    ) -> Result<(), StoreError> {
        let mut relevant_input_notes =
            self.get_relevant_sync_input_notes(&state_sync_update).await.unwrap();

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

        // Serialize data for updating state sync and block header
        let block_num_as_str = block_header.block_num().to_string();

        // Serialize data for updating spent notes
        let nullifiers_as_str = nullifiers
            .iter()
            .map(|nullifier_update| nullifier_update.nullifier.to_hex())
            .collect();
        let nullifier_block_nums_as_str = nullifiers
            .iter()
            .map(|nullifier_update| nullifier_update.block_num.to_string())
            .collect();

        // Serialize data for updating block header
        let block_header_as_bytes = block_header.to_bytes();
        let new_mmr_peaks_as_bytes = new_mmr_peaks.peaks().to_vec().to_bytes();

        // Serialize data for updating chain MMR nodes
        let mut serialized_node_ids = Vec::new();
        let mut serialized_nodes = Vec::new();
        for (id, node) in new_authentication_nodes.iter() {
            let serialized_data = serialize_chain_mmr_node(*id, *node)?;
            serialized_node_ids.push(serialized_data.id);
            serialized_nodes.push(serialized_data.node);
        }

        // Serialize data for updating committed notes
        let output_note_ids_as_str: Vec<String> = committed_notes
            .updated_output_notes()
            .iter()
            .map(|(note_id, _)| note_id.inner().to_hex())
            .collect();

        let output_note_inclusion_proofs_as_bytes: Vec<Vec<u8>> = committed_notes
            .updated_output_notes()
            .iter()
            .map(|(_, inclusion_proof)| {
                let block_num = inclusion_proof.location().block_num();
                let note_index = inclusion_proof.location().node_index_in_block();

                // Create a NoteInclusionProof and serialize it to JSON, handle errors with `?`
                NoteInclusionProof::new(block_num, note_index, inclusion_proof.note_path().clone())
                    .unwrap()
                    .to_bytes()
            })
            .collect::<Vec<Vec<u8>>>();
        let flattened_nested_vec_output_note_inclusion_proofs =
            flatten_nested_u8_vec(output_note_inclusion_proofs_as_bytes);

        let input_note_ids_as_str: Vec<String> = committed_notes
            .updated_input_notes()
            .iter()
            .map(|input_note| input_note.id().inner().to_hex())
            .collect();

        // TODO: LOP INTO idxdb_apply_state_sync call
        for input_note in committed_notes.updated_input_notes().iter() {
            let inclusion_proof = input_note.proof().ok_or(StoreError::DatabaseError(
                "Input note doesn't have inclusion proof".to_string(),
            ))?;
            let metadata: &miden_objects::notes::NoteMetadata = input_note.note().metadata();

            if let Some(input_note_record) =
                relevant_input_notes.iter_mut().find(|n| n.id() == input_note.id())
            {
                let inclusion_proof_received = input_note_record
                    .inclusion_proof_received(inclusion_proof.clone(), *metadata)?;
                let block_header_received =
                    input_note_record.block_header_received(block_header)?;

                if inclusion_proof_received || block_header_received {
                    upsert_input_note_tx(input_note_record.clone()).await.unwrap();
                }
            }
        }

        // TODO: LOP INTO idxdb_apply_state_sync call
        // Commit new public notes
        for note in committed_notes.new_public_notes() {
            let details = note.note().into();

            let input_note_record = InputNoteRecord::new(
                details,
                None,
                CommittedNoteState {
                    metadata: *note.note().metadata(),
                    inclusion_proof: note
                        .proof()
                        .expect("New public note should be authenticated")
                        .clone(),
                    block_note_root: block_header.note_root(),
                }
                .into(),
            );

            upsert_input_note_tx(input_note_record).await.unwrap();
        }

        // Serialize data for updating committed transactions
        let transactions_to_commit_block_nums_as_str = committed_transactions
            .iter()
            .map(|tx_update| tx_update.block_num.to_string())
            .collect();
        let transactions_to_commit_as_str: Vec<String> = committed_transactions
            .iter()
            .map(|tx_update| tx_update.transaction_id.to_string())
            .collect();

        // TODO: LOP INTO idxdb_apply_state_sync call
        // Update onchain accounts on the db that have been updated onchain
        for account in updated_onchain_accounts {
            update_account(&account.clone()).await.unwrap();
        }

        let promise = idxdb_apply_state_sync(
            block_num_as_str,
            nullifiers_as_str,
            nullifier_block_nums_as_str,
            block_header_as_bytes,
            new_mmr_peaks_as_bytes,
            block_has_relevant_notes,
            serialized_node_ids,
            serialized_nodes,
            output_note_ids_as_str,
            flattened_nested_vec_output_note_inclusion_proofs,
            input_note_ids_as_str,
            transactions_to_commit_as_str,
            transactions_to_commit_block_nums_as_str,
        );
        JsFuture::from(promise).await.unwrap();

        Ok(())
    }

    /// Get the input notes from the store that are relevant to the state sync update. Secifically,
    /// notes that were updated and nullified during the sync.
    async fn get_relevant_sync_input_notes(
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
            self.get_input_notes(NoteFilter::List(updated_input_note_ids)).await.unwrap();

        let nullifiers = nullifiers
            .iter()
            .map(|nullifier_update| nullifier_update.nullifier)
            .collect::<Vec<_>>();
        let nullified_notes =
            self.get_input_notes(NoteFilter::Nullifiers(nullifiers)).await.unwrap();

        let mut relevant_notes = updated_input_notes;
        relevant_notes.extend(nullified_notes);

        Ok(relevant_notes)
    }
}

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::notes::{NoteInclusionProof, NoteTag};
use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::*;

use super::{
    chain_data::utils::serialize_chain_mmr_node, notes::utils::insert_input_note_tx,
    transactions::utils::update_account, WebStore,
};
use crate::{store::StoreError, sync::StateSyncUpdate};

mod js_bindings;
use js_bindings::*;

mod models;
use models::*;

impl WebStore {
    pub(crate) async fn get_note_tags(&self) -> Result<Vec<NoteTag>, StoreError> {
        let promise = idxdb_get_note_tags();
        let js_value = JsFuture::from(promise).await.unwrap();
        let tags_idxdb: NoteTagsIdxdbObject = from_value(js_value).unwrap();

        let tags: Vec<NoteTag> = serde_json::from_str(&tags_idxdb.tags).unwrap();

        Ok(tags)
    }

    pub(super) async fn get_sync_height(&self) -> Result<u32, StoreError> {
        let promise = idxdb_get_sync_height();
        let js_value = JsFuture::from(promise).await.unwrap();
        let block_num_idxdb: SyncHeightIdxdbObject = from_value(js_value).unwrap();

        let block_num_as_u32: u32 = block_num_idxdb.block_num.parse::<u32>().unwrap();
        Ok(block_num_as_u32)
    }

    pub(super) async fn add_note_tag(&self, tag: NoteTag) -> Result<bool, StoreError> {
        let mut tags = self.get_note_tags().await.unwrap();
        if tags.contains(&tag) {
            return Ok(false);
        }
        tags.push(tag);
        let tags = serde_json::to_string(&tags).map_err(StoreError::InputSerializationError)?;

        let promise = idxdb_add_note_tag(tags);
        JsFuture::from(promise).await.unwrap();

        let tag_as_u32 = u32::from(tag);
        let tag_as_str = tag_as_u32.to_string();
        let promise = idxdb_update_ignored_notes_for_tag(tag_as_str);
        JsFuture::from(promise).await.unwrap();

        Ok(true)
    }

    pub(super) async fn remove_note_tag(&self, tag: NoteTag) -> Result<bool, StoreError> {
        let mut tags = self.get_note_tags().await?;
        if let Some(index_of_tag) = tags.iter().position(|&tag_candidate| tag_candidate == tag) {
            tags.remove(index_of_tag);

            let tags = serde_json::to_string(&tags).map_err(StoreError::InputSerializationError)?;

            let promise = idxdb_add_note_tag(tags);
            JsFuture::from(promise).await.unwrap();
            return Ok(true);
        }

        Ok(false)
    }

    pub(super) async fn apply_state_sync(
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
        let block_header_as_str =
            serde_json::to_string(&block_header).map_err(StoreError::InputSerializationError)?;
        let new_mmr_peaks_as_str = serde_json::to_string(&new_mmr_peaks.peaks().to_vec())
            .map_err(StoreError::InputSerializationError)?;

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
        let output_note_inclusion_proofs_as_str: Vec<String> = committed_notes
            .updated_output_notes()
            .iter()
            .map(|(_, inclusion_proof)| {
                let block_num = inclusion_proof.location().block_num();
                let note_index = inclusion_proof.location().node_index_in_block();

                // Create a NoteInclusionProof and serialize it to JSON, handle errors with `?`
                let proof = NoteInclusionProof::new(
                    block_num,
                    note_index,
                    inclusion_proof.note_path().clone(),
                )
                .unwrap();

                serde_json::to_string(&proof).unwrap()
            })
            .collect();

        let input_note_ids_as_str: Vec<String> = committed_notes
            .updated_input_notes()
            .iter()
            .map(|input_note| input_note.id().inner().to_hex())
            .collect();
        let input_note_inclusion_proofs_as_str: Vec<String> = committed_notes
            .updated_input_notes()
            .iter()
            .map(|input_note| {
                let inclusion_proof =
                    input_note.proof().expect("Expected a valid NoteInclusionProof");
                serde_json::to_string(inclusion_proof).unwrap()
            })
            .collect();
        let input_note_metadatas_as_str: Vec<String> = committed_notes
            .updated_input_notes()
            .iter()
            .map(|input_note| {
                let metadata = input_note.note().metadata();
                serde_json::to_string(metadata).unwrap()
            })
            .collect();

        // TODO: LOP INTO idxdb_apply_state_sync call
        // Commit new public notes
        for note in committed_notes.new_public_notes() {
            insert_input_note_tx(note.clone().into()).await.unwrap();
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
            block_header_as_str,
            new_mmr_peaks_as_str,
            block_has_relevant_notes,
            serialized_node_ids,
            serialized_nodes,
            output_note_ids_as_str,
            output_note_inclusion_proofs_as_str,
            input_note_ids_as_str,
            input_note_inclusion_proofs_as_str,
            input_note_metadatas_as_str,
            transactions_to_commit_as_str,
            transactions_to_commit_block_nums_as_str,
        );
        JsFuture::from(promise).await.unwrap();

        Ok(())
    }
}

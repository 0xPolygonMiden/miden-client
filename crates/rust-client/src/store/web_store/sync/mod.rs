use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    account::AccountId,
    block::BlockNumber,
    note::{NoteId, NoteTag},
    transaction::TransactionId,
};
use miden_tx::utils::{Deserializable, Serializable};
use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::JsFuture;

use super::{
    WebStore,
    account::{lock_account, utils::update_account},
    chain_data::utils::serialize_chain_mmr_node,
    note::utils::apply_note_updates_tx,
};
use crate::{
    store::{StoreError, TransactionFilter},
    sync::{NoteTagRecord, NoteTagSource, StateSyncUpdate},
};

mod js_bindings;
use js_bindings::{
    idxdb_add_note_tag, idxdb_apply_state_sync, idxdb_get_note_tags, idxdb_get_sync_height,
    idxdb_remove_note_tag,
};

mod models;
use models::{NoteTagIdxdbObject, SyncHeightIdxdbObject};

mod flattened_vec;
use flattened_vec::flatten_nested_u8_vec;

impl WebStore {
    pub(crate) async fn get_note_tags(&self) -> Result<Vec<NoteTagRecord>, StoreError> {
        let promise = idxdb_get_note_tags();
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to get note tags: {js_error:?}"))
        })?;
        let tags_idxdb: Vec<NoteTagIdxdbObject> = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

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

    pub(super) async fn get_sync_height(&self) -> Result<BlockNumber, StoreError> {
        let promise = idxdb_get_sync_height();
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to get sync height: {js_error:?}"))
        })?;
        let block_num_idxdb: SyncHeightIdxdbObject = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        let block_num_as_u32: u32 = block_num_idxdb.block_num.parse::<u32>().unwrap();
        Ok(block_num_as_u32.into())
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
        JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to add note tag: {js_error:?}"))
        })?;

        Ok(true)
    }

    pub(super) async fn remove_note_tag(&self, tag: NoteTagRecord) -> Result<usize, StoreError> {
        let (source_note_id, source_account_id) = match tag.source {
            NoteTagSource::Note(note_id) => (Some(note_id.to_hex()), None),
            NoteTagSource::Account(account_id) => (None, Some(account_id.to_hex())),
            NoteTagSource::User => (None, None),
        };

        let promise = idxdb_remove_note_tag(tag.tag.to_bytes(), source_note_id, source_account_id);
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to remove note tag: {js_error:?}"))
        })?;
        let removed_tags: usize = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        Ok(removed_tags)
    }

    pub(super) async fn apply_state_sync(
        &self,
        state_sync_update: StateSyncUpdate,
    ) -> Result<(), StoreError> {
        let StateSyncUpdate {
            block_num,
            block_updates,
            note_updates,
            transaction_updates,
            account_updates,
        } = state_sync_update;

        // Serialize data for updating block header
        let mut block_headers_as_bytes = vec![];
        let mut new_mmr_peaks_as_bytes = vec![];
        let mut block_nums_as_str = vec![];
        let mut block_has_relevant_notes = vec![];

        for (block_header, has_client_notes, mmr_peaks) in block_updates.block_headers() {
            block_headers_as_bytes.push(block_header.to_bytes());
            new_mmr_peaks_as_bytes.push(mmr_peaks.peaks().to_vec().to_bytes());
            block_nums_as_str.push(block_header.block_num().to_string());
            block_has_relevant_notes.push(u8::from(*has_client_notes));
        }

        // Serialize data for updating chain MMR nodes
        let mut serialized_node_ids = Vec::new();
        let mut serialized_nodes = Vec::new();
        for (id, node) in block_updates.new_authentication_nodes() {
            let serialized_data = serialize_chain_mmr_node(*id, *node)?;
            serialized_node_ids.push(serialized_data.id);
            serialized_nodes.push(serialized_data.node);
        }

        // TODO: LOP INTO idxdb_apply_state_sync call
        // Update notes
        apply_note_updates_tx(&note_updates).await?;

        // Tags to remove
        let note_tags_to_remove_as_str: Vec<String> = note_updates
            .updated_input_notes()
            .filter_map(|note_update| {
                let note = note_update.inner();
                if note.is_committed() {
                    Some(
                        note.metadata()
                            .expect("Committed notes should have metadata")
                            .tag()
                            .to_string(),
                    )
                } else {
                    None
                }
            })
            .collect();

        // Serialize data for updating committed transactions
        let transactions_to_commit_block_nums_as_str = transaction_updates
            .committed_transactions()
            .iter()
            .map(|tx_update| tx_update.block_num.to_string())
            .collect();
        let transactions_to_commit_as_str: Vec<String> = transaction_updates
            .committed_transactions()
            .iter()
            .map(|tx_update| tx_update.transaction_id.to_string())
            .collect();
        let transactions_to_discard_as_str: Vec<String> = transaction_updates
            .discarded_transactions()
            .iter()
            .map(TransactionId::to_string)
            .collect();

        // TODO: LOP INTO idxdb_apply_state_sync call
        // Update public accounts on the db that have been updated onchain
        for account in account_updates.updated_public_accounts() {
            update_account(&account.clone()).await.map_err(|err| {
                StoreError::DatabaseError(format!("failed to update account: {err:?}"))
            })?;
        }

        for (account_id, digest) in account_updates.mismatched_private_accounts() {
            // Mismatched digests may be due to stale network data. If the mismatched digest is
            // tracked in the db and corresponds to the mismatched account, it means we
            // got a past update and shouldn't lock the account.
            if let Some(account) = self.get_account_header_by_commitment(*digest).await? {
                if account.id() == *account_id {
                    continue;
                }
            }

            lock_account(account_id).await.map_err(|err| {
                StoreError::DatabaseError(format!("failed to lock account: {err:?}"))
            })?;
        }

        let account_states_to_rollback = self
            .get_transactions(TransactionFilter::Ids(
                transaction_updates.discarded_transactions().to_vec(),
            ))
            .await?
            .iter()
            .map(|tx_record| tx_record.final_account_state)
            .collect::<Vec<_>>();

        // Remove the account states that are originated from the discarded transactions
        self.undo_account_states(&account_states_to_rollback).await?;

        let promise = idxdb_apply_state_sync(
            block_num.to_string(),
            flatten_nested_u8_vec(block_headers_as_bytes),
            block_nums_as_str,
            flatten_nested_u8_vec(new_mmr_peaks_as_bytes),
            block_has_relevant_notes,
            serialized_node_ids,
            serialized_nodes,
            note_tags_to_remove_as_str,
            transactions_to_commit_as_str,
            transactions_to_commit_block_nums_as_str,
            transactions_to_discard_as_str,
        );
        JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to apply state sync: {js_error:?}"))
        })?;

        Ok(())
    }
}

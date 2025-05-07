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
    chain_data::utils::{SerializedPartialBlockchainNodeData, serialize_partial_blockchain_node},
    note::utils::apply_note_updates_tx,
};
use crate::{
    note::NoteUpdates,
    store::{StoreError, TransactionFilter},
    sync::{NoteTagRecord, NoteTagSource, StateSyncUpdate},
};

mod js_bindings;
use js_bindings::{
    idxdb_add_note_tag, idxdb_apply_state_sync, idxdb_discard_transactions, idxdb_get_note_tags,
    idxdb_get_sync_height, idxdb_remove_note_tag,
};

mod models;
use models::{NoteTagIdxdbObject, SyncHeightIdxdbObject};

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
            block_header,
            block_has_relevant_notes,
            new_mmr_peaks,
            new_authentication_nodes,
            note_updates,
            account_updates,
            tags_to_remove,
            transaction_updates,
        } = state_sync_update;

        // Serialize data for updating state sync and block header
        let block_num_as_str = block_header.block_num().to_string();

        // Serialize data for updating block header
        let block_header_as_bytes = block_header.to_bytes();
        let new_mmr_peaks_as_bytes = new_mmr_peaks.peaks().to_vec().to_bytes();

        // Serialize data for updating partial blockchain nodes
        let mut serialized_node_ids = Vec::new();
        let mut serialized_nodes = Vec::new();
        for (id, node) in &new_authentication_nodes {
            let SerializedPartialBlockchainNodeData { id, node } =
                serialize_partial_blockchain_node(*id, *node)?;
            serialized_node_ids.push(id);
            serialized_nodes.push(node);
        }

        // TODO: LOP INTO idxdb_apply_state_sync call
        // Update notes
        apply_note_updates_tx(&note_updates).await?;

        // Tags to remove
        let note_tags_to_remove_as_str: Vec<String> = tags_to_remove
            .iter()
            .filter_map(|tag_record| {
                if let NoteTagSource::Note(note_id) = tag_record.source {
                    Some(note_id.to_hex())
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

        for (account_id, _) in account_updates.mismatched_private_accounts() {
            lock_account(account_id).await.map_err(|err| {
                StoreError::DatabaseError(format!("failed to lock account: {err:?}"))
            })?;
        }

        let promise = idxdb_apply_state_sync(
            block_num_as_str,
            block_header_as_bytes,
            new_mmr_peaks_as_bytes,
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

    pub(super) async fn apply_nullifiers(
        &self,
        note_updates: NoteUpdates,
        transactions_to_discard: Vec<TransactionId>,
    ) -> Result<(), StoreError> {
        // First we need the `transaction` entries from the `transactions` table that matches the
        // `transactions_to_discard`
        let transactions_records_to_discard = self
            .get_transactions(TransactionFilter::Ids(transactions_to_discard.clone()))
            .await?;

        apply_note_updates_tx(&note_updates).await?;
        let transactions_ids_as_str: Vec<String> =
            transactions_to_discard.iter().map(ToString::to_string).collect();

        let promise = idxdb_discard_transactions(transactions_ids_as_str);

        let final_account_states = transactions_records_to_discard
            .iter()
            .map(|tx_record| tx_record.final_account_state)
            .collect::<Vec<_>>();

        // Remove the account states that are originated from the discarded transactions
        self.undo_account_states(&final_account_states).await?;

        JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to discard transactions: {js_error:?}"))
        })?;

        Ok(())
    }
}

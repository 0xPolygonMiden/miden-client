use std::collections::BTreeMap;

use crate::{
    client::{
        rpc::TransactionUpdate,
        transactions::{TransactionRecord, TransactionResult, TransactionStatus},
    },
    errors::StoreError,
    store::TransactionFilter,
};
use miden_objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    transaction::{OutputNotes, TransactionScript},
    Digest, Felt,
};
use miden_tx::utils::Deserializable;
use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::*;

use super::{
    notes::utils::{insert_input_note_tx, insert_output_note_tx, update_note_consumer_tx_id},
    WebStore
};

mod js_bindings;
use js_bindings::*;

mod models;
use models::*;

pub mod utils;
use utils::*;

impl WebStore {
    pub async fn get_transactions(
        &self,
        filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError> {
        let filter_as_str = match filter {
            TransactionFilter::All => "All",
            TransactionFilter::Uncomitted => "Uncomitted",
        };

        let promise = idxdb_get_transactions(filter_as_str.to_string());
        let js_value = JsFuture::from(promise).await.unwrap();
        let transactions_idxdb: Vec<TransactionIdxdbObject> = from_value(js_value).unwrap();

        let transaction_records: Result<Vec<TransactionRecord>, StoreError> = transactions_idxdb
            .into_iter()
            .map(|tx_idxdb| {
                let native_account_id = AccountId::from_hex(&tx_idxdb.account_id).unwrap();
                let block_num_as_u32: u32 = tx_idxdb.block_num.parse::<u32>().unwrap();
                let commit_height_as_u32: Option<u32> =
                    tx_idxdb.commit_height.map(|height| height.parse::<u32>().unwrap());

                let id: Digest = tx_idxdb.id.try_into()?;
                let init_account_state: Digest = tx_idxdb.init_account_state.try_into()?;

                let final_account_state: Digest = tx_idxdb.final_account_state.try_into()?;

                let input_note_nullifiers: Vec<Digest> =
                    serde_json::from_str(&tx_idxdb.input_notes)
                        .map_err(StoreError::JsonDataDeserializationError)?;

                let output_notes = OutputNotes::read_from_bytes(&tx_idxdb.output_notes)?;

                let transaction_script: Option<TransactionScript> =
                    if tx_idxdb.script_hash.is_some() {
                        let script_hash = tx_idxdb
                            .script_hash
                            .map(|hash| Digest::read_from_bytes(&hash))
                            .transpose()?
                            .expect("Script hash should be included in the row");

                        let script_program = tx_idxdb
                            .script_program
                            .map(|program| ProgramAst::from_bytes(&program))
                            .transpose()?
                            .expect("Script program should be included in the row");

                        let script_inputs = tx_idxdb
                            .script_inputs
                            .map(|hash| serde_json::from_str::<BTreeMap<Digest, Vec<Felt>>>(&hash))
                            .transpose()
                            .map_err(StoreError::JsonDataDeserializationError)?
                            .expect("Script inputs should be included in the row");

                        let tx_script = TransactionScript::from_parts(
                            script_program,
                            script_hash,
                            script_inputs.into_iter().map(|(k, v)| (k.into(), v)),
                        )?;

                        Some(tx_script)
                    } else {
                        None
                    };

                let transaction_status = commit_height_as_u32
                    .map_or(TransactionStatus::Pending, TransactionStatus::Committed);

                Ok(TransactionRecord {
                    id: id.into(),
                    account_id: native_account_id,
                    init_account_state,
                    final_account_state,
                    input_note_nullifiers,
                    output_notes,
                    transaction_script,
                    block_num: block_num_as_u32,
                    transaction_status,
                })
            })
            .collect();

        transaction_records
    }

    pub async fn apply_transaction(&self, tx_result: TransactionResult) -> Result<(), StoreError> {
        let transaction_id = tx_result.executed_transaction().id();
        let account_id = tx_result.executed_transaction().account_id();
        let account_delta = tx_result.account_delta();

        let (mut account, _seed) = self.get_account(account_id).await.unwrap();

        account.apply_delta(account_delta).map_err(StoreError::AccountError)?;

        // Save only input notes that we care for (based on the note screener assessment)
        let created_input_notes = tx_result.relevant_notes().to_vec();

        // Save all output notes
        let created_output_notes = tx_result
            .created_notes()
            .iter()
            .cloned()
            .filter_map(|output_note| output_note.try_into().ok())
            .collect::<Vec<_>>();

        let consumed_note_ids =
            tx_result.consumed_notes().iter().map(|note| note.id()).collect::<Vec<_>>();

        // Transaction Data
        insert_proven_transaction_data(tx_result).await.unwrap();

        // Account Data
        update_account(&account).await.unwrap();

        // Updates for notes
        for note in created_input_notes {
            insert_input_note_tx(&note).await?;
        }

        for note in &created_output_notes {
            insert_output_note_tx(note).await?;
        }

        for note_id in consumed_note_ids {
            update_note_consumer_tx_id(note_id, transaction_id).await?;
        }

        Ok(())
    }

    /// This function is not used in this crate, rather it is used in the 'miden-client' crate.
    /// https://github.com/0xPolygonMiden/miden-client/blob/c273847726ed325d2e627e4db18bf9f3ab8c28ba/src/store/sqlite_store/sync.rs#L168
    /// It is duplicated here due to its reliance on the store.
    #[allow(dead_code)]
    pub(crate) async fn mark_transactions_as_committed(
        &self,
        transactions_to_commit: &[TransactionUpdate],
    ) -> Result<usize, StoreError> {
        let block_nums_as_str = transactions_to_commit
            .iter()
            .map(|tx_update| tx_update.block_num.to_string())
            .collect::<Vec<String>>();
        let transaction_ids_as_str = transactions_to_commit
            .iter()
            .map(|tx_update| tx_update.transaction_id.to_string())
            .collect::<Vec<String>>();

        let promise =
            idxdb_mark_transactions_as_committed(block_nums_as_str, transaction_ids_as_str);
        let js_value = JsFuture::from(promise).await.unwrap();
        let result: usize = from_value(js_value).unwrap();

        Ok(result)
    }
}

use alloc::{string::ToString, vec::Vec};

use miden_objects::{
    account::AccountId,
    block::BlockNumber,
    transaction::{OutputNotes, TransactionScript},
    Digest,
};
use miden_tx::utils::Deserializable;
use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::*;

use super::{accounts::utils::update_account, notes::utils::apply_note_updates_tx, WebStore};
use crate::{
    store::{StoreError, TransactionFilter},
    transactions::{TransactionRecord, TransactionStatus, TransactionStoreUpdate},
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
                let block_num: BlockNumber = tx_idxdb.block_num.parse::<u32>().unwrap().into();
                let commit_height: Option<BlockNumber> =
                    tx_idxdb.commit_height.map(|height| height.parse::<u32>().unwrap().into());

                let id: Digest = tx_idxdb.id.try_into()?;
                let init_account_state: Digest = tx_idxdb.init_account_state.try_into()?;

                let final_account_state: Digest = tx_idxdb.final_account_state.try_into()?;

                let input_note_nullifiers: Vec<Digest> =
                    Vec::<Digest>::read_from_bytes(&tx_idxdb.input_notes)?;

                let output_notes = OutputNotes::read_from_bytes(&tx_idxdb.output_notes)?;

                let transaction_script: Option<TransactionScript> =
                    if tx_idxdb.script_hash.is_some() {
                        let tx_script = tx_idxdb
                            .tx_script
                            .map(|script| TransactionScript::read_from_bytes(&script))
                            .transpose()?
                            .expect("Transaction script should be included in the row");

                        Some(tx_script)
                    } else {
                        None
                    };

                let transaction_status =
                    commit_height.map_or(TransactionStatus::Pending, TransactionStatus::Committed);

                Ok(TransactionRecord {
                    id: id.into(),
                    account_id: native_account_id,
                    init_account_state,
                    final_account_state,
                    input_note_nullifiers,
                    output_notes,
                    transaction_script,
                    block_num,
                    transaction_status,
                })
            })
            .collect();

        transaction_records
    }

    pub async fn apply_transaction(
        &self,
        tx_update: TransactionStoreUpdate,
    ) -> Result<(), StoreError> {
        // Transaction Data
        insert_proven_transaction_data(tx_update.executed_transaction()).await?;

        // Account Data
        update_account(tx_update.updated_account()).await.unwrap();

        // Updates for notes
        apply_note_updates_tx(tx_update.note_updates()).await?;

        for tag_record in tx_update.new_tags() {
            self.add_note_tag(*tag_record).await?;
        }

        Ok(())
    }
}

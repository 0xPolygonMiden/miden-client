use crate::native_code::{store::NativeTransactionFilter, transactions};

use wasm_bindgen_futures::*;
use serde_wasm_bindgen::from_value;

use super::WebStore;

mod js_bindings;
use js_bindings::*;

mod models;
use models::*;

mod utils;
use utils::*;

impl WebStore {
    // pub async fn get_transactions(
    //     &mut self,
    //     filter: NativeTransactionFilter
    // ) -> Result<Vec<TransactionRecord>, ()> {
    //     let filter_as_str = match filter {
    //         NativeTransactionFilter::All => "All",
    //         NativeTransactionFilter::Uncomitted => "Uncomitted",
    //     };

    //     let promise = idxdb_get_transactions(filter_as_str.to_string());
    //     let js_value = JsFuture::from(promise).await?;
    //     let transactions_idxdb: Vec<TransactionIdxdbObject> = from_value(js_value).unwrap();

    //     let transaction_records: Result<Vec<TransactionRecord>, ()> = transactions_idxdb.into_iter().map(|tx_idxdb| {
    //         let native_account_id = AccountId::from_hex(&tx_idxdb.account_id).map_err(|_err| ())?;
    //         let block_num_as_u32: u32 = tx_idxdb.block_num.parse::<u32>().unwrap();
    //         let commit_height_as_u32: Option<u32> = match tx_idxdb.commit_height {
    //             Some(height) => Some(height.parse::<u32>().unwrap()),
    //             _ => None,
    //         };

    //         let id: Digest = tx_idxdb.id.try_into()?;
    //         let init_account_state: Digest = init_account_state.try_into()?;

    //         let final_account_state: Digest = final_account_state.try_into()?;

    //         let input_note_nullifiers: Vec<Digest> =
    //             serde_json::from_str(&input_notes).map_err(StoreError::JsonDataDeserializationError)?;

    //         let output_notes: OutputNotes<OutputNote> = OutputNotes::read_from_bytes(&output_notes)?;

    //         let transaction_script: Option<TransactionScript> = if script_hash.is_some() {
    //             let script_hash = script_hash
    //                 .map(|hash| Digest::read_from_bytes(&hash))
    //                 .transpose()?
    //                 .expect("Script hash should be included in the row");

    //             let script_program = script_program
    //                 .map(|program| ProgramAst::from_bytes(&program))
    //                 .transpose()?
    //                 .expect("Script program should be included in the row");

    //             let script_inputs = script_inputs
    //                 .map(|hash| serde_json::from_str::<BTreeMap<Digest, Vec<Felt>>>(&hash))
    //                 .transpose()
    //                 .map_err(StoreError::JsonDataDeserializationError)?
    //                 .expect("Script inputs should be included in the row");

    //             let tx_script = TransactionScript::from_parts(
    //                 script_program,
    //                 script_hash,
    //                 script_inputs.into_iter().map(|(k, v)| (k.into(), v)),
    //             )?;

    //             Some(tx_script)
    //         } else {
    //             None
    //         };

    //         let transaction_status =
    //             commit_height_as_u32.map_or(TransactionStatus::Pending, TransactionStatus::Committed);

    //         Ok(TransactionRecord {
    //             id: id.into(),
    //             native_account_id,
    //             init_account_state,
    //             final_account_state,
    //             input_note_nullifiers,
    //             output_notes,
    //             transaction_script,
    //             block_num_as_u32,
    //             transaction_status,
    //         })
    //     }).collect();

    //     transaction_records
    // }

    // pub async fn apply_transaction(
    //     &mut self,
    //     tx_result: TransactionResult
    // ) -> Result<(), ()> {
    //     let account_id = tx_result.executed_transaction().account_id();
    //     let account_delta = tx_result.account_delta();

    //     let (mut account, _seed) = self.get_account(account_id).await?;

    //     account.apply_delta(account_delta).map_err()?;

    //     let created_notes = tx_result
    //         .created_notes()
    //         .iter()
    //         .map(|note| InputNoteRecord::from(note.clone()))
    //         .collect::<Vec<_>>();

    //     // Transaction Data
    //     insert_proven_transaction_data(tx_result).await?;

    //     // Account Data
    //     update_account(account).await?;

    //     // Updates for notes

    //     // TODO: see if we should filter the input notes we store to keep notes we can consume with
    //     // existing accounts
    //     for note in &created_notes {
    //         insert_input_note_tx(note).await?;
    //     }

    //     for note in &created_notes {
    //         insert_output_note_tx(note).await?;
    //     }

    //     Ok(())
    // }

    // pub(crate) async fn mark_transactions_as_committed(
    //     &mut self,
    //     block_num: u32,
    //     transactions_to_commit: &[TransactionId]
    // ) -> Result<usize, ()> {
    //     let block_num_as_str = block_num.to_string();
    //     let transactions_to_commit_as_str: Vec<String> = transactions_to_commit.iter().map(|tx_id| tx_id.inner().into()).collect();

    //     let promise = idxdb_mark_transactions_as_committed(block_num_as_str, transactions_to_commit_as_str);
    //     let js_value = JsFuture::from(promise).await?;
    //     let result: usize = from_value(js_value).unwrap();
        
    //     Ok(result)
    // }
}


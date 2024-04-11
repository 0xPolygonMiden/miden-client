use wasm_bindgen_futures::*;

use super::js_bindings::*;

// TYPES
// ================================================================================================

// type SerializedTransactionData = (
//     String,
//     String,
//     String,
//     String,
//     String,
//     Vec<u8>,
//     Option<Vec<u8>>,
//     Option<Vec<u8>>,
//     Option<String>,
//     String,
//     Option<String>,
// );

// ================================================================================================

// pub async fn insert_proven_transaction_data(
//     transaction_result: TransactionResult
// ) -> Result<(), ()> {
//     let (
//         transaction_id,
//         account_id,
//         init_account_state,
//         final_account_state,
//         input_notes,
//         output_notes,
//         script_program,
//         script_hash,
//         script_inputs,
//         block_num,
//         committed
//     ) = serialize_transaction_data(transaction_result)?;

//     if let Some(hash) = script_hash.clone() {
//         let promise = idxdb_insert_transaction_script(script_hash, script_program);
//         let result = JsFuture::from(promise).await;
//     }

//     let promise = idxdb_insert_proven_transaction_data(
//         transaction_id,
//         account_id,
//         init_account_state,
//         final_account_state,
//         input_notes,
//         output_notes,
//         script_program,
//         script_hash,
//         script_inputs,
//         block_num,
//         committed
//     );
//     let result = JsFuture::from(promise).await;

//     match result {
//         Ok(_) => Ok(()),
//         Err(_) => Err(()),
//     }
// }

// pub(super) async fn serialize_transaction_data(
//     transaction_result: TransactionResult
// ) -> Result<SerializedTransactionData, ()> {
//     let executed_transaction = transaction_result.executed_transaction();
//     let transaction_id: String = executed_transaction.id().inner().into();

//     let account_id_as_str: String = AccountId::to_hex(executed_transaction.account_id());
//     let init_account_state = &executed_transaction.initial_account().hash().to_string();
//     let final_account_state = &executed_transaction.final_account().hash().to_string();

//     // TODO: Double check if saving nullifiers as input notes is enough
//     let nullifiers: Vec<Digest> =
//         executed_transaction.input_notes().iter().map(|x| x.id().inner()).collect();

//     let input_notes =
//         serde_json::to_string(&nullifiers).map_err(|err| ())?;

//     let output_notes = executed_transaction.output_notes();

//     // TODO: Scripts should be in their own tables and only identifiers should be stored here
//     let transaction_args = transaction_result.transaction_arguments();
//     let mut script_program = None;
//     let mut script_hash = None;
//     let mut script_inputs = None;

//     if let Some(tx_script) = transaction_args.tx_script() {
//         script_program = Some(tx_script.code().to_bytes(AstSerdeOptions {
//             serialize_imports: true,
//         }));
//         script_hash = Some(tx_script.hash().to_bytes());
//         script_inputs = Some(
//             serde_json::to_string(&tx_script.inputs())
//                 .map_err(|err| ())?,
//         );
//     }

//     Ok((
//         transaction_id,
//         account_id_as_str,
//         init_account_state.to_owned(),
//         final_account_state.to_owned(),
//         input_notes,
//         output_notes.to_bytes(),
//         script_program,
//         script_hash,
//         script_inputs,
//         transaction_result.block_num().to_string(),
//         None,
//     ))
// }

// pub(super) async fn update_account(
//     new_account_state: Account,
// ) -> Result<(), ()> {
//     insert_account_storage(new_account_state.storage())?;
//     insert_account_asset_vault(new_account_state.vault())?;
//     insert_account_record(&new_account_state, None)
// }
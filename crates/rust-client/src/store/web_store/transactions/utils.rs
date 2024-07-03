use miden_objects::{
    accounts::Account, assembly::AstSerdeOptions, transaction::ToInputNoteCommitments, Digest,
};
use miden_tx::utils::Serializable;
use wasm_bindgen_futures::*;

use super::js_bindings::*;
use crate::{
    store::{
        web_store::accounts::utils::{
            insert_account_asset_vault, insert_account_record, insert_account_storage,
        },
        StoreError,
    },
    transactions::TransactionResult,
};

// TYPES
// ================================================================================================

pub struct SerializedTransactionData {
    pub transaction_id: String,
    pub account_id: String,
    pub init_account_state: String,
    pub final_account_state: String,
    pub input_notes: String,
    pub output_notes: Vec<u8>,
    pub script_program: Option<Vec<u8>>,
    pub script_hash: Option<Vec<u8>>,
    pub script_inputs: Option<String>,
    pub block_num: String,
    pub commit_height: Option<String>,
}

// ================================================================================================

pub async fn insert_proven_transaction_data(
    transaction_result: TransactionResult,
) -> Result<(), StoreError> {
    let serialized_data = serialize_transaction_data(transaction_result)?;

    if let Some(hash) = serialized_data.script_hash.clone() {
        let promise = idxdb_insert_transaction_script(hash, serialized_data.script_program.clone());
        JsFuture::from(promise).await.unwrap();
    }

    let promise = idxdb_insert_proven_transaction_data(
        serialized_data.transaction_id,
        serialized_data.account_id,
        serialized_data.init_account_state,
        serialized_data.final_account_state,
        serialized_data.input_notes,
        serialized_data.output_notes,
        serialized_data.script_hash.clone(),
        serialized_data.script_inputs.clone(),
        serialized_data.block_num,
        serialized_data.commit_height,
    );
    JsFuture::from(promise).await.unwrap();

    Ok(())
}

pub(super) fn serialize_transaction_data(
    transaction_result: TransactionResult,
) -> Result<SerializedTransactionData, StoreError> {
    let executed_transaction = transaction_result.executed_transaction();
    let transaction_id: String = executed_transaction.id().inner().into();

    let account_id_as_str: String = executed_transaction.account_id().to_string();
    let init_account_state = &executed_transaction.initial_account().hash().to_string();
    let final_account_state = &executed_transaction.final_account().hash().to_string();

    // TODO: Double check if saving nullifiers as input notes is enough
    let nullifiers: Vec<Digest> = executed_transaction
        .input_notes()
        .iter()
        .map(|x| x.nullifier().inner())
        .collect();

    let input_notes =
        serde_json::to_string(&nullifiers).map_err(StoreError::InputSerializationError)?;

    let output_notes = executed_transaction.output_notes();

    // TODO: Scripts should be in their own tables and only identifiers should be stored here
    let transaction_args = transaction_result.transaction_arguments();
    let mut script_program = None;
    let mut script_hash = None;
    let mut script_inputs = None;

    if let Some(tx_script) = transaction_args.tx_script() {
        script_program =
            Some(tx_script.code().to_bytes(AstSerdeOptions { serialize_imports: true }));
        script_hash = Some(tx_script.hash().to_bytes());
        script_inputs = Some(
            serde_json::to_string(&tx_script.inputs())
                .map_err(StoreError::InputSerializationError)?,
        );
    }

    Ok(SerializedTransactionData {
        transaction_id,
        account_id: account_id_as_str,
        init_account_state: init_account_state.to_owned(),
        final_account_state: final_account_state.to_owned(),
        input_notes,
        output_notes: output_notes.to_bytes(),
        script_program,
        script_hash,
        script_inputs,
        block_num: transaction_result.block_num().to_string(),
        commit_height: None,
    })
}

pub async fn update_account(new_account_state: &Account) -> Result<(), ()> {
    let _ = insert_account_storage(new_account_state.storage()).await;
    let _ = insert_account_asset_vault(new_account_state.vault()).await;
    insert_account_record(new_account_state, None).await
}

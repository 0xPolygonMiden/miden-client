use miden_client::{client::transactions::TransactionResult, errors::StoreError};
use miden_objects::{
    accounts::Account, assembly::AstSerdeOptions, transaction::ToInputNoteCommitments, Digest,
};
use miden_tx::utils::Serializable;
use wasm_bindgen_futures::*;

use super::js_bindings::*;
// use crate::native_code::{errors::StoreError, transactions::TransactionResult};
use crate::web_client::store::accounts::utils::{
    insert_account_asset_vault, insert_account_record, insert_account_storage,
};

// TYPES
// ================================================================================================

type SerializedTransactionData = (
    String,
    String,
    String,
    String,
    String,
    Vec<u8>,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
    Option<String>,
    String,
    Option<String>,
);

// ================================================================================================

pub async fn insert_proven_transaction_data(
    transaction_result: TransactionResult,
) -> Result<(), StoreError> {
    let (
        transaction_id,
        account_id,
        init_account_state,
        final_account_state,
        input_notes,
        output_notes,
        script_program,
        script_hash,
        script_inputs,
        block_num,
        committed,
    ) = serialize_transaction_data(transaction_result)?;

    if let Some(hash) = script_hash.clone() {
        let promise = idxdb_insert_transaction_script(hash, script_program.clone());
        JsFuture::from(promise).await.unwrap();
    }

    let promise = idxdb_insert_proven_transaction_data(
        transaction_id,
        account_id,
        init_account_state,
        final_account_state,
        input_notes,
        output_notes,
        script_hash.clone(),
        script_inputs.clone(),
        block_num,
        committed,
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

    Ok((
        transaction_id,
        account_id_as_str,
        init_account_state.to_owned(),
        final_account_state.to_owned(),
        input_notes,
        output_notes.to_bytes(),
        script_program,
        script_hash,
        script_inputs,
        transaction_result.block_num().to_string(),
        None,
    ))
}

pub async fn update_account(new_account_state: &Account) -> Result<(), ()> {
    let _ = insert_account_storage(new_account_state.storage()).await;
    let _ = insert_account_asset_vault(new_account_state.vault()).await;
    insert_account_record(new_account_state, None).await
}

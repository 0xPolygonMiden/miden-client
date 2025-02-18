use alloc::{
    borrow::ToOwned,
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    transaction::{ExecutedTransaction, ToInputNoteCommitments},
    Digest,
};
use miden_tx::utils::Serializable;
use wasm_bindgen_futures::JsFuture;

use super::js_bindings::{idxdb_insert_proven_transaction_data, idxdb_insert_transaction_script};
use crate::store::StoreError;

// TYPES
// ================================================================================================

pub struct SerializedTransactionData {
    pub transaction_id: String,
    pub account_id: String,
    pub init_account_state: String,
    pub final_account_state: String,
    pub input_notes: Vec<u8>,
    pub output_notes: Vec<u8>,
    pub script_hash: Option<Vec<u8>>,
    pub tx_script: Option<Vec<u8>>,
    pub block_num: String,
    pub commit_height: Option<String>,
}

// ================================================================================================

pub async fn insert_proven_transaction_data(
    executed_transaction: &ExecutedTransaction,
) -> Result<(), StoreError> {
    let serialized_data = serialize_transaction_data(executed_transaction);

    if let Some(hash) = serialized_data.script_hash.clone() {
        let promise = idxdb_insert_transaction_script(hash, serialized_data.tx_script);
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
        serialized_data.block_num,
        serialized_data.commit_height,
    );
    JsFuture::from(promise).await.unwrap();

    Ok(())
}

pub(super) fn serialize_transaction_data(
    executed_transaction: &ExecutedTransaction,
) -> SerializedTransactionData {
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

    let input_notes = nullifiers.to_bytes();

    let output_notes = executed_transaction.output_notes();

    // TODO: Scripts should be in their own tables and only identifiers should be stored here
    let transaction_args = executed_transaction.tx_args();
    let mut script_hash = None;
    let mut tx_script = None;

    if let Some(script) = transaction_args.tx_script() {
        script_hash = Some(script.hash().to_bytes());
        tx_script = Some(script.to_bytes());
    }

    SerializedTransactionData {
        transaction_id,
        account_id: account_id_as_str,
        init_account_state: init_account_state.to_owned(),
        final_account_state: final_account_state.to_owned(),
        input_notes,
        output_notes: output_notes.to_bytes(),
        script_hash,
        tx_script,
        block_num: executed_transaction.block_header().block_num().to_string(),
        commit_height: None,
    }
}

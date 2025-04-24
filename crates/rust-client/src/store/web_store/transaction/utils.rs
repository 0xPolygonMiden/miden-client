use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    Digest,
    transaction::{ExecutedTransaction, ToInputNoteCommitments},
};
use miden_tx::utils::Serializable;
use wasm_bindgen_futures::JsFuture;

use super::js_bindings::{idxdb_insert_proven_transaction_data, idxdb_insert_transaction_script};
use crate::{store::StoreError, transaction::TransactionDetails};

// TYPES
// ================================================================================================

pub struct SerializedTransactionData {
    pub transaction_id: String,
    pub details: Vec<u8>,
    pub script_root: Option<Vec<u8>>,
    pub tx_script: Option<Vec<u8>>,
    pub block_num: String,
    pub commit_height: Option<String>,
}

// ================================================================================================

pub async fn insert_proven_transaction_data(
    executed_transaction: &ExecutedTransaction,
) -> Result<(), StoreError> {
    let serialized_data = serialize_transaction_data(executed_transaction);

    if let Some(root) = serialized_data.script_root.clone() {
        let promise = idxdb_insert_transaction_script(root, serialized_data.tx_script);
        JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to insert script: {js_error:?}"))
        })?;
    }

    let promise = idxdb_insert_proven_transaction_data(
        serialized_data.transaction_id,
        serialized_data.details,
        serialized_data.script_root.clone(),
        serialized_data.block_num,
        serialized_data.commit_height,
    );
    JsFuture::from(promise).await.map_err(|js_error| {
        StoreError::DatabaseError(format!("failed to insert transaction data: {js_error:?}"))
    })?;

    Ok(())
}

pub(super) fn serialize_transaction_data(
    executed_transaction: &ExecutedTransaction,
) -> SerializedTransactionData {
    let transaction_id: String = executed_transaction.id().inner().into();

    // TODO: Double check if saving nullifiers as input notes is enough
    let nullifiers: Vec<Digest> = executed_transaction
        .input_notes()
        .iter()
        .map(|x| x.nullifier().inner())
        .collect();

    // TODO: Scripts should be in their own tables and only identifiers should be stored here
    let transaction_args = executed_transaction.tx_args();
    let mut script_root = None;
    let mut tx_script = None;

    if let Some(script) = transaction_args.tx_script() {
        script_root = Some(script.root().to_bytes());
        tx_script = Some(script.to_bytes());
    }

    let details = TransactionDetails {
        account_id: executed_transaction.account_id(),
        init_account_state: executed_transaction.initial_account().commitment(),
        final_account_state: executed_transaction.final_account().commitment(),
        input_note_nullifiers: nullifiers,
        output_notes: executed_transaction.output_notes().clone(),
        block_num: executed_transaction.block_header().block_num(),
        expiration_block_num: executed_transaction.expiration_block_num(),
    };

    SerializedTransactionData {
        transaction_id,
        details: details.to_bytes(),
        script_root,
        tx_script,
        block_num: executed_transaction.block_header().block_num().to_string(),
        commit_height: None,
    }
}

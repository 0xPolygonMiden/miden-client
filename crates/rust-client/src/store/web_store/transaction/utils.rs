use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    Digest,
    transaction::{ExecutedTransaction, ToInputNoteCommitments, TransactionScript},
};
use miden_tx::utils::Serializable;
use wasm_bindgen_futures::JsFuture;

use super::js_bindings::{idxdb_insert_transaction_script, idxdb_upsert_transaction_record};
use crate::{
    store::StoreError,
    transaction::{TransactionDetails, TransactionRecord, TransactionStatus},
};

// TYPES
// ================================================================================================

pub struct SerializedTransactionData {
    pub id: String,
    pub details: Vec<u8>,
    pub script_root: Option<Vec<u8>>,
    pub tx_script: Option<Vec<u8>>,
    pub block_num: String,
    pub commit_height: Option<String>,
    pub discard_cause: Option<Vec<u8>>,
}

// ================================================================================================

pub async fn insert_proven_transaction_data(
    executed_transaction: &ExecutedTransaction,
) -> Result<(), StoreError> {
    // Build transaction record
    let nullifiers: Vec<Digest> = executed_transaction
        .input_notes()
        .iter()
        .map(|x| x.nullifier().inner())
        .collect();

    let output_notes = executed_transaction.output_notes();

    let details = TransactionDetails {
        account_id: executed_transaction.account_id(),
        init_account_state: executed_transaction.initial_account().commitment(),
        final_account_state: executed_transaction.final_account().commitment(),
        input_note_nullifiers: nullifiers,
        output_notes: output_notes.clone(),
        block_num: executed_transaction.block_header().block_num(),
        expiration_block_num: executed_transaction.expiration_block_num(),
    };

    let transaction_record = TransactionRecord::new(
        executed_transaction.id(),
        details,
        executed_transaction.tx_args().tx_script().cloned(),
        TransactionStatus::Pending,
    );

    upsert_transaction_record(&transaction_record).await?;

    Ok(())
}

/// Serializes the transaction record into a format suitable for storage in the database.
pub(super) fn serialize_transaction_record(
    transaction_record: &TransactionRecord,
) -> SerializedTransactionData {
    let transaction_id: String = transaction_record.id.inner().into();

    let script_root = transaction_record.script.as_ref().map(|script| script.root().to_bytes());
    let tx_script = transaction_record.script.as_ref().map(TransactionScript::to_bytes);

    let (commit_height, discard_cause) = match &transaction_record.status {
        TransactionStatus::Pending => (None, None),
        TransactionStatus::Committed(block_num) => (Some(block_num.as_u32().to_string()), None),
        TransactionStatus::Discarded(cause) => (None, Some(cause.to_bytes())),
    };

    SerializedTransactionData {
        id: transaction_id,
        script_root,
        tx_script,
        details: transaction_record.details.to_bytes(),
        block_num: transaction_record.details.block_num.as_u32().to_string(),
        commit_height,
        discard_cause,
    }
}

/// Updates the transaction record in the database, inserting it if it doesn't exist.
pub(crate) async fn upsert_transaction_record(
    transaction: &TransactionRecord,
) -> Result<(), StoreError> {
    let serialized_data = serialize_transaction_record(transaction);

    if let Some(root) = serialized_data.script_root.clone() {
        let promise = idxdb_insert_transaction_script(root, serialized_data.tx_script);
        JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to insert script: {js_error:?}"))
        })?;
    }

    let promise = idxdb_upsert_transaction_record(
        serialized_data.id,
        serialized_data.details,
        serialized_data.script_root.clone(),
        serialized_data.block_num,
        serialized_data.commit_height,
        serialized_data.discard_cause,
    );
    JsFuture::from(promise).await.map_err(|js_error| {
        StoreError::DatabaseError(format!("failed to insert transaction data: {js_error:?}"))
    })?;

    Ok(())
}

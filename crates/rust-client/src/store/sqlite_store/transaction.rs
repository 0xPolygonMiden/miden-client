#![allow(clippy::items_after_statements)]

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use std::rc::Rc;

use miden_objects::{
    Digest,
    block::BlockNumber,
    crypto::utils::{Deserializable, Serializable},
    transaction::{ToInputNoteCommitments, TransactionScript},
};
use rusqlite::{Connection, Transaction, params, types::Value};

use super::{
    SqliteStore, account::update_account, note::apply_note_updates_tx, sync::add_note_tag_tx,
};
use crate::{
    insert_sql,
    store::{StoreError, TransactionFilter},
    subst,
    transaction::{
        DiscardCause, TransactionDetails, TransactionRecord, TransactionStatus,
        TransactionStoreUpdate,
    },
};

pub(crate) const UPSERT_TRANSACTION_QUERY: &str = insert_sql!(
    transactions {
        id,
        details,
        script_root,
        block_num,
        commit_height,
        discard_cause
    } | REPLACE
);

pub(crate) const INSERT_TRANSACTION_SCRIPT_QUERY: &str =
    insert_sql!(transaction_scripts { script_root, script } | IGNORE);

// TRANSACTIONS FILTERS
// ================================================================================================

impl TransactionFilter {
    /// Returns a [String] containing the query for this Filter.
    pub fn to_query(&self) -> String {
        const QUERY: &str = "SELECT tx.id, script.script, tx.details, tx.commit_height, tx.discard_cause \
            FROM transactions AS tx LEFT JOIN transaction_scripts AS script ON tx.script_root = script.script_root";
        match self {
            TransactionFilter::All => QUERY.to_string(),
            TransactionFilter::Uncommitted => format!("{QUERY} WHERE tx.commit_height IS NULL"),
            TransactionFilter::Ids(_) => {
                // Use SQLite's array parameter binding
                format!("{QUERY} WHERE tx.id IN rarray(?)")
            },
            TransactionFilter::ExpiredBefore(block_num) => {
                format!(
                    "{QUERY} WHERE tx.block_num < {} AND tx.discard_cause IS NULL AND tx.commit_height IS NULL",
                    block_num.as_u32()
                )
            },
        }
    }
}

// TRANSACTIONS
// ================================================================================================

struct SerializedTransactionData {
    /// Transaction ID
    id: String,
    /// Script root
    script_root: Option<Vec<u8>>,
    /// Transaction script
    tx_script: Option<Vec<u8>>,
    /// Transaction details
    details: Vec<u8>,
    /// Block number
    block_num: u32,
    /// Commit height
    commit_height: Option<u32>,
    /// Cause for discarding the transaction, if applicable
    discard_cause: Option<Vec<u8>>,
}

struct SerializedTransactionParts {
    /// Transaction ID
    id: String,
    /// Transaction script
    tx_script: Option<Vec<u8>>,
    /// Transaction details
    details: Vec<u8>,
    /// Block number of the block at which the transaction was included in the chain.
    commit_height: Option<u32>,
    /// Cause for discarding the transaction, if applicable
    discard_cause: Option<Vec<u8>>,
}

impl SqliteStore {
    /// Retrieves tracked transactions, filtered by [`TransactionFilter`].
    pub fn get_transactions(
        conn: &mut Connection,
        filter: &TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError> {
        match filter {
            TransactionFilter::Ids(ids) => {
                // Convert transaction IDs to strings for the array parameter
                let id_strings =
                    ids.iter().map(|id| Value::Text(id.to_string())).collect::<Vec<_>>();

                // Create a prepared statement and bind the array parameter
                conn.prepare(&filter.to_query())?
                    .query_map(params![Rc::new(id_strings)], parse_transaction_columns)?
                    .map(|result| Ok(result?).and_then(parse_transaction))
                    .collect::<Result<Vec<TransactionRecord>, _>>()
            },
            _ => {
                // For other filters, no parameters are needed
                conn.prepare(&filter.to_query())?
                    .query_map([], parse_transaction_columns)?
                    .map(|result| Ok(result?).and_then(parse_transaction))
                    .collect::<Result<Vec<TransactionRecord>, _>>()
            },
        }
    }

    /// Inserts a transaction and updates the current state based on the `tx_result` changes.
    pub fn apply_transaction(
        conn: &mut Connection,
        tx_update: &TransactionStoreUpdate,
    ) -> Result<(), StoreError> {
        let tx = conn.transaction()?;

        // Build transaction record
        let executed_transaction = tx_update.executed_transaction();

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

        // Insert transaction data
        upsert_transaction_record(&tx, &transaction_record)?;

        // Account Data
        update_account(&tx, tx_update.updated_account())?;

        // Note Updates
        apply_note_updates_tx(&tx, tx_update.note_updates())?;

        for tag_record in tx_update.new_tags() {
            add_note_tag_tx(&tx, tag_record)?;
        }

        tx.commit()?;

        Ok(())
    }
}

/// Updates the transaction record in the database, inserting it if it doesn't exist.
pub(crate) fn upsert_transaction_record(
    tx: &Transaction<'_>,
    transaction: &TransactionRecord,
) -> Result<(), StoreError> {
    let SerializedTransactionData {
        id,
        script_root,
        tx_script,
        details,
        block_num,
        commit_height,
        discard_cause,
    } = serialize_transaction_data(transaction);

    if let Some(root) = script_root.clone() {
        tx.execute(INSERT_TRANSACTION_SCRIPT_QUERY, params![root, tx_script])?;
    }

    tx.execute(
        UPSERT_TRANSACTION_QUERY,
        params![id, details, script_root, block_num, commit_height, discard_cause],
    )?;

    Ok(())
}

/// Serializes the transaction record into a format suitable for storage in the database.
fn serialize_transaction_data(transaction_record: &TransactionRecord) -> SerializedTransactionData {
    let transaction_id: String = transaction_record.id.inner().into();

    let script_root = transaction_record.script.as_ref().map(|script| script.root().to_bytes());
    let tx_script = transaction_record.script.as_ref().map(TransactionScript::to_bytes);

    let (commit_height, discard_cause) = match &transaction_record.status {
        TransactionStatus::Pending => (None, None),
        TransactionStatus::Committed(block_num) => (Some(block_num.as_u32()), None),
        TransactionStatus::Discarded(cause) => (None, Some(cause.to_bytes())),
    };

    SerializedTransactionData {
        id: transaction_id,
        script_root,
        tx_script,
        details: transaction_record.details.to_bytes(),
        block_num: transaction_record.details.block_num.as_u32(),
        commit_height,
        discard_cause,
    }
}

fn parse_transaction_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedTransactionParts, rusqlite::Error> {
    let id: String = row.get(0)?;
    let tx_script: Option<Vec<u8>> = row.get(1)?;
    let details: Vec<u8> = row.get(2)?;
    let commit_height: Option<u32> = row.get(3)?;
    let discard_cause: Option<Vec<u8>> = row.get(4)?;

    Ok(SerializedTransactionParts {
        id,
        tx_script,
        details,
        commit_height,
        discard_cause,
    })
}

/// Parse a transaction from the provided parts.
fn parse_transaction(
    serialized_transaction: SerializedTransactionParts,
) -> Result<TransactionRecord, StoreError> {
    let SerializedTransactionParts {
        id,
        tx_script,
        details,
        commit_height,
        discard_cause,
    } = serialized_transaction;

    let id: Digest = id.try_into()?;

    let script: Option<TransactionScript> = tx_script
        .map(|script| TransactionScript::read_from_bytes(&script))
        .transpose()?;

    let status = if let Some(cause) = discard_cause {
        let cause = DiscardCause::read_from_bytes(&cause)?;
        TransactionStatus::Discarded(cause)
    } else {
        let commit_height = commit_height.map(BlockNumber::from);
        commit_height.map_or(TransactionStatus::Pending, TransactionStatus::Committed)
    };

    Ok(TransactionRecord {
        id: id.into(),
        details: TransactionDetails::read_from_bytes(&details)?,
        script,
        status,
    })
}

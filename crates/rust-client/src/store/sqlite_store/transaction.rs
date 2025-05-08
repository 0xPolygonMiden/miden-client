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
    transaction::{ExecutedTransaction, ToInputNoteCommitments, TransactionId, TransactionScript},
};
use rusqlite::{Connection, Transaction, params, types::Value};
use tracing::info;

use super::{
    SqliteStore, account::update_account, note::apply_note_updates_tx, sync::add_note_tag_tx,
};
use crate::{
    insert_sql,
    rpc::domain::transaction::TransactionUpdate,
    store::{StoreError, TransactionFilter},
    subst,
    transaction::{
        TransactionDetails, TransactionRecord, TransactionStatus, TransactionStoreUpdate,
    },
};

pub(crate) const INSERT_TRANSACTION_QUERY: &str = insert_sql!(transactions {
    id,
    details,
    script_root,
    block_num,
    commit_height,
    discarded
});

pub(crate) const INSERT_TRANSACTION_SCRIPT_QUERY: &str =
    insert_sql!(transaction_scripts { script_root, script } | IGNORE);

// TRANSACTIONS FILTERS
// ================================================================================================

impl TransactionFilter {
    /// Returns a [String] containing the query for this Filter.
    pub fn to_query(&self) -> String {
        const QUERY: &str = "SELECT tx.id, script.script, tx.details, tx.commit_height, tx.discarded \
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
                    "{QUERY} WHERE tx.block_num < {} AND tx.discarded = false AND tx.commit_height IS NULL",
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
    /// Discarded flag
    discarded: bool,
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
    /// Indicates whether the transaction has been discarded.
    discarded: bool,
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

        // Transaction Data
        insert_proven_transaction_data(&tx, tx_update.executed_transaction())?;

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

    /// Set the provided transactions as committed.
    ///
    /// # Errors
    ///
    /// This function can return an error if any of the updates to the transactions within the
    /// database transaction fail.
    pub(crate) fn mark_transactions_as_committed(
        tx: &Transaction<'_>,
        transactions_to_commit: &[TransactionUpdate],
    ) -> Result<usize, StoreError> {
        let mut rows = 0;
        for transaction_update in transactions_to_commit {
            const QUERY: &str = "UPDATE transactions set commit_height=? where id=?";
            rows += tx.execute(
                QUERY,
                params![
                    Some(transaction_update.block_num),
                    transaction_update.transaction_id.to_string()
                ],
            )?;
        }
        info!("Marked {} transactions as committed", rows);

        Ok(rows)
    }

    /// Set the provided transactions as committed.
    ///
    /// # Errors
    ///
    /// This function can return an error if any of the updates to the transactions within the
    /// database transaction fail.
    pub(crate) fn mark_transactions_as_discarded(
        tx: &Transaction<'_>,
        transactions_to_discard: &[TransactionId],
    ) -> Result<usize, StoreError> {
        let mut rows = 0;
        for transaction_id in transactions_to_discard {
            const QUERY: &str = "UPDATE transactions set discarded=true where id=?";
            rows += tx.execute(QUERY, params![transaction_id.to_string()])?;
        }

        Ok(rows)
    }
}

pub(super) fn insert_proven_transaction_data(
    tx: &Transaction<'_>,
    executed_transaction: &ExecutedTransaction,
) -> Result<(), StoreError> {
    let SerializedTransactionData {
        id,
        script_root,
        tx_script,
        details,
        block_num,
        commit_height,
        discarded,
    } = serialize_transaction_data(executed_transaction);

    if let Some(root) = script_root.clone() {
        tx.execute(INSERT_TRANSACTION_SCRIPT_QUERY, params![root, tx_script])?;
    }

    tx.execute(
        INSERT_TRANSACTION_QUERY,
        params![id, details, script_root, block_num, commit_height, discarded,],
    )?;

    Ok(())
}

fn serialize_transaction_data(
    executed_transaction: &ExecutedTransaction,
) -> SerializedTransactionData {
    let transaction_id: String = executed_transaction.id().inner().into();

    // TODO: Double check if saving nullifiers as input notes is enough
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

    info!("Transaction ID: {}", executed_transaction.id().inner());
    info!("Transaction account ID: {}", executed_transaction.account_id());

    // TODO: Scripts should be in their own tables and only identifiers should be stored here
    let transaction_args = executed_transaction.tx_args();
    let tx_script = transaction_args.tx_script().map(TransactionScript::to_bytes);
    let script_root = transaction_args.tx_script().map(|script| script.root().to_bytes());

    SerializedTransactionData {
        id: transaction_id,
        script_root,
        tx_script,
        details: details.to_bytes(),
        block_num: executed_transaction.block_header().block_num().as_u32(),
        commit_height: None,
        discarded: false,
    }
}

fn parse_transaction_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedTransactionParts, rusqlite::Error> {
    let id: String = row.get(0)?;
    let tx_script: Option<Vec<u8>> = row.get(1)?;
    let details: Vec<u8> = row.get(2)?;
    let commit_height: Option<u32> = row.get(3)?;
    let discarded: bool = row.get(4)?;

    Ok(SerializedTransactionParts {
        id,
        tx_script,
        details,
        commit_height,
        discarded,
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
        discarded,
    } = serialized_transaction;

    let id: Digest = id.try_into()?;

    let script: Option<TransactionScript> = tx_script
        .map(|script| TransactionScript::read_from_bytes(&script))
        .transpose()?;

    let status = if discarded {
        TransactionStatus::Discarded
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

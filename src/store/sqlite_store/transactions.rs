use alloc::collections::BTreeMap;

use miden_objects::{
    accounts::AccountId,
    assembly::{AstSerdeOptions, ProgramAst},
    crypto::utils::{Deserializable, Serializable},
    transaction::{OutputNotes, ToNullifier, TransactionId, TransactionScript},
    Digest, Felt,
};
use rusqlite::{params, Transaction};
use tracing::info;

use super::{
    accounts::update_account,
    notes::{insert_input_note_tx, insert_output_note_tx, update_note_consumer_tx_id},
    SqliteStore,
};
use crate::{
    client::transactions::{TransactionRecord, TransactionResult, TransactionStatus},
    errors::StoreError,
    store::TransactionFilter,
};

pub(crate) const INSERT_TRANSACTION_QUERY: &str =
    "INSERT INTO transactions (id, account_id, init_account_state, final_account_state, \
    input_notes, output_notes, script_hash, script_inputs, block_num, commit_height) \
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

pub(crate) const INSERT_TRANSACTION_SCRIPT_QUERY: &str =
    "INSERT OR IGNORE INTO transaction_scripts (script_hash, program) \
    VALUES (?, ?)";

// TRANSACTIONS FILTERS
// ================================================================================================

impl TransactionFilter {
    /// Returns a [String] containing the query for this Filter
    pub fn to_query(&self) -> String {
        const QUERY: &str = "SELECT tx.id, tx.account_id, tx.init_account_state, tx.final_account_state, \
            tx.input_notes, tx.output_notes, tx.script_hash, script.program, tx.script_inputs, tx.block_num, tx.commit_height \
            FROM transactions AS tx LEFT JOIN transaction_scripts AS script ON tx.script_hash = script.script_hash";
        match self {
            TransactionFilter::All => QUERY.to_string(),
            TransactionFilter::Uncomitted => format!("{QUERY} WHERE tx.commit_height IS NULL"),
        }
    }
}

// TRANSACTIONS
// ================================================================================================

type SerializedTransactionData = (
    String,
    i64,
    String,
    String,
    String,
    Vec<u8>,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
    Option<String>,
    u32,
    Option<u32>,
);

impl SqliteStore {
    /// Retrieves tracked transactions, filtered by [TransactionFilter].
    pub fn get_transactions(
        &self,
        filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError> {
        self.db()
            .prepare(&filter.to_query())?
            .query_map([], parse_transaction_columns)
            .expect("no binding parameters used in query")
            .map(|result| Ok(result?).and_then(parse_transaction))
            .collect::<Result<Vec<TransactionRecord>, _>>()
    }

    /// Inserts a transaction and updates the current state based on the `tx_result` changes
    pub fn apply_transaction(&self, tx_result: TransactionResult) -> Result<(), StoreError> {
        let transaction_id = tx_result.executed_transaction().id();
        let account_id = tx_result.executed_transaction().account_id();
        let account_delta = tx_result.account_delta();

        let (mut account, _seed) = self.get_account(account_id)?;

        account.apply_delta(account_delta).map_err(StoreError::AccountError)?;

        // Save only input notes that we care for (based on the note screener assessment)
        let created_input_notes = tx_result.relevant_notes().to_vec();

        // Save all output notes
        let created_output_notes = tx_result
            .created_notes()
            .iter()
            .cloned()
            .filter_map(|output_note| output_note.try_into().ok())
            .collect::<Vec<_>>();

        let consumed_note_ids =
            tx_result.consumed_notes().iter().map(|note| note.id()).collect::<Vec<_>>();

        let mut db = self.db();
        let tx = db.transaction()?;

        // Transaction Data
        insert_proven_transaction_data(&tx, tx_result)?;

        // Account Data
        update_account(&tx, &account)?;

        // Updates for notes
        for note in created_input_notes {
            insert_input_note_tx(&tx, &note)?;
        }

        for note in &created_output_notes {
            insert_output_note_tx(&tx, note)?;
        }

        for note_id in consumed_note_ids {
            update_note_consumer_tx_id(&tx, note_id, transaction_id)?;
        }

        tx.commit()?;

        Ok(())
    }

    /// Set the provided transactions as committed
    ///
    /// # Errors
    ///
    /// This function can return an error if any of the updates to the transactions within the
    /// database transaction fail.
    pub(crate) fn mark_transactions_as_committed(
        tx: &Transaction<'_>,
        block_num: u32,
        transactions_to_commit: &[TransactionId],
    ) -> Result<usize, StoreError> {
        let mut rows = 0;
        for transaction_id in transactions_to_commit {
            const QUERY: &str = "UPDATE transactions set commit_height=? where id=?";
            rows += tx.execute(QUERY, params![Some(block_num), transaction_id.to_string()])?;
        }
        info!("Marked {} transactions as committed", rows);

        Ok(rows)
    }
}

pub(super) fn insert_proven_transaction_data(
    tx: &Transaction<'_>,
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
        tx.execute(INSERT_TRANSACTION_SCRIPT_QUERY, params![hash, script_program])?;
    }

    tx.execute(
        INSERT_TRANSACTION_QUERY,
        params![
            transaction_id,
            account_id,
            init_account_state,
            final_account_state,
            input_notes,
            output_notes,
            script_hash,
            script_inputs,
            block_num,
            committed,
        ],
    )?;

    Ok(())
}

pub(super) fn serialize_transaction_data(
    transaction_result: TransactionResult,
) -> Result<SerializedTransactionData, StoreError> {
    let executed_transaction = transaction_result.executed_transaction();
    let transaction_id: String = executed_transaction.id().inner().into();
    let account_id: u64 = executed_transaction.account_id().into();
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

    info!("Transaction ID: {}", executed_transaction.id().inner());
    info!("Transaction account ID: {}", executed_transaction.account_id());

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
        account_id as i64,
        init_account_state.to_owned(),
        final_account_state.to_owned(),
        input_notes,
        output_notes.to_bytes(),
        script_program,
        script_hash,
        script_inputs,
        transaction_result.block_num(),
        None,
    ))
}

fn parse_transaction_columns(
    row: &rusqlite::Row<'_>,
) -> Result<SerializedTransactionData, rusqlite::Error> {
    let id: String = row.get(0)?;
    let account_id: i64 = row.get(1)?;
    let init_account_state: String = row.get(2)?;
    let final_account_state: String = row.get(3)?;
    let input_notes: String = row.get(4)?;
    let output_notes: Vec<u8> = row.get(5)?;
    let script_hash: Option<Vec<u8>> = row.get(6)?;
    let script_program: Option<Vec<u8>> = row.get(7)?;
    let script_inputs: Option<String> = row.get(8)?;
    let block_num: u32 = row.get(9)?;
    let commit_height: Option<u32> = row.get(10)?;

    Ok((
        id,
        account_id,
        init_account_state,
        final_account_state,
        input_notes,
        output_notes,
        script_hash,
        script_program,
        script_inputs,
        block_num,
        commit_height,
    ))
}

/// Parse a transaction from the provided parts.
fn parse_transaction(
    serialized_transaction: SerializedTransactionData,
) -> Result<TransactionRecord, StoreError> {
    let (
        id,
        account_id,
        init_account_state,
        final_account_state,
        input_notes,
        output_notes,
        script_hash,
        script_program,
        script_inputs,
        block_num,
        commit_height,
    ) = serialized_transaction;
    let account_id = AccountId::try_from(account_id as u64)?;
    let id: Digest = id.try_into()?;
    let init_account_state: Digest = init_account_state.try_into()?;

    let final_account_state: Digest = final_account_state.try_into()?;

    let input_note_nullifiers: Vec<Digest> =
        serde_json::from_str(&input_notes).map_err(StoreError::JsonDataDeserializationError)?;

    let output_notes = OutputNotes::read_from_bytes(&output_notes)?;

    let transaction_script: Option<TransactionScript> = if script_hash.is_some() {
        let script_hash = script_hash
            .map(|hash| Digest::read_from_bytes(&hash))
            .transpose()?
            .expect("Script hash should be included in the row");

        let script_program = script_program
            .map(|program| ProgramAst::from_bytes(&program))
            .transpose()?
            .expect("Script program should be included in the row");

        let script_inputs = script_inputs
            .map(|hash| serde_json::from_str::<BTreeMap<Digest, Vec<Felt>>>(&hash))
            .transpose()
            .map_err(StoreError::JsonDataDeserializationError)?
            .expect("Script inputs should be included in the row");

        let tx_script = TransactionScript::from_parts(
            script_program,
            script_hash,
            script_inputs.into_iter().map(|(k, v)| (k.into(), v)),
        )?;

        Some(tx_script)
    } else {
        None
    };

    let transaction_status =
        commit_height.map_or(TransactionStatus::Pending, TransactionStatus::Committed);

    Ok(TransactionRecord {
        id: id.into(),
        account_id,
        init_account_state,
        final_account_state,
        input_note_nullifiers,
        output_notes,
        transaction_script,
        block_num,
        transaction_status,
    })
}

use crate::{
    client::transactions::{TransactionResult, TransactionStub},
    errors::StoreError,
    store::notes::InputNoteRecord,
};
use crypto::{
    utils::{collections::BTreeMap, Deserializable, Serializable},
    Felt,
};

use super::Store;
use objects::{
    accounts::AccountId,
    assembly::{AstSerdeOptions, ProgramAst},
    notes::{NoteEnvelope, NoteId},
    transaction::{OutputNotes, TransactionScript},
    Digest,
};
use rusqlite::{params, Transaction};

pub(crate) const INSERT_TRANSACTION_QUERY: &str =
    "INSERT INTO transactions (id, account_id, init_account_state, final_account_state, \
    input_notes, output_notes, script_hash, script_inputs, block_num, committed, commit_height) \
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

pub(crate) const INSERT_TRANSACTION_SCRIPT_QUERY: &str =
    "INSERT OR IGNORE INTO transaction_scripts (script_hash, program) \
    VALUES (?, ?)";

// TRANSACTIONS FILTERS
// ================================================================================================

pub enum TransactionFilter {
    All,
    Uncomitted,
}

impl TransactionFilter {
    pub fn to_query(&self) -> String {
        const QUERY: &str = "SELECT tx.id, tx.account_id, tx.init_account_state, tx.final_account_state, \
            tx.input_notes, tx.output_notes, tx.script_hash, script.program, tx.script_inputs, tx.block_num, tx.committed, tx.commit_height \
            FROM transactions AS tx LEFT JOIN transaction_scripts AS script ON tx.script_hash = script.script_hash";
        match self {
            TransactionFilter::All => QUERY.to_string(),
            TransactionFilter::Uncomitted => format!("{QUERY} WHERE tx.committed=false"),
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
    bool,
    u32,
);

impl Store {
    /// Retrieves all executed transactions from the database
    pub fn get_transactions(
        &self,
        transaction_filter: TransactionFilter,
    ) -> Result<Vec<TransactionStub>, StoreError> {
        self.db
            .prepare(&transaction_filter.to_query())
            .map_err(StoreError::QueryError)?
            .query_map([], parse_transaction_columns)
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(parse_transaction)
            })
            .collect::<Result<Vec<TransactionStub>, _>>()
    }

    pub fn insert_transaction_data(
        &mut self,
        tx_result: TransactionResult,
    ) -> Result<(), StoreError> {
        let account_id = tx_result.executed_transaction().account_id();
        let account_delta = tx_result.account_delta();

        let (mut account, seed) = self.get_account_by_id(account_id)?;

        account
            .apply_delta(account_delta)
            .map_err(StoreError::AccountError)?;

        let created_notes = tx_result
            .created_notes()
            .iter()
            .map(|note| InputNoteRecord::from(note.clone()))
            .collect::<Vec<_>>();

        let tx = self
            .db
            .transaction()
            .map_err(StoreError::TransactionError)?;

        // Transaction Data
        Self::insert_proven_transaction_data(&tx, tx_result)?;

        // Account Data
        Self::insert_account_storage(&tx, account.storage())?;
        Self::insert_account_asset_vault(&tx, account.vault())?;
        Self::insert_account_record(&tx, &account, seed)?;

        // Updates for notes
        for note in created_notes {
            Self::insert_input_note_tx(&tx, &note)?;
        }

        tx.commit().map_err(StoreError::TransactionError)?;

        Ok(())
    }

    fn insert_proven_transaction_data(
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
            commit_height,
        ) = serialize_transaction_data(transaction_result)?;

        if let Some(hash) = script_hash.clone() {
            tx.execute(
                INSERT_TRANSACTION_SCRIPT_QUERY,
                params![hash, script_program],
            )
            .map(|_| ())
            .map_err(StoreError::QueryError)?;
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
                commit_height,
            ],
        )
        .map(|_| ())
        .map_err(StoreError::QueryError)?;

        Ok(())
    }

    /// Updates transactions as committed if the input `note_ids` belongs to one uncommitted transaction
    pub(crate) fn mark_transactions_as_committed_by_note_id(
        uncommitted_transactions: &[TransactionStub],
        note_ids: &[NoteId],
        block_num: u32,
        tx: &Transaction<'_>,
    ) -> Result<usize, StoreError> {
        let updated_transactions: Vec<&TransactionStub> = uncommitted_transactions
            .iter()
            .filter(|t| {
                t.output_notes
                    .iter()
                    .any(|n| note_ids.contains(&n.note_id()))
            })
            .collect();

        let mut rows = 0;
        for transaction in updated_transactions {
            const QUERY: &str =
                "UPDATE transactions set committed=true, commit_height=? where id=?";
            rows += tx
                .execute(QUERY, params![block_num, transaction.id.to_string()])
                .map_err(StoreError::QueryError)?;
        }

        Ok(rows)
    }
}

pub(crate) fn serialize_transaction_data(
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
        .map(|x| x.id().inner())
        .collect();

    let input_notes =
        serde_json::to_string(&nullifiers).map_err(StoreError::InputSerializationError)?;

    let output_notes = executed_transaction.output_notes();

    // TODO: Add proper logging
    println!("transaction id {}", executed_transaction.id().inner());
    println!(
        "transaction account id: {}",
        executed_transaction.account_id()
    );

    // TODO: Scripts should be in their own tables and only identifiers should be stored here
    let mut script_program = None;
    let mut script_hash = None;
    let mut script_inputs = None;

    if let Some(tx_script) = transaction_result.transaction_script() {
        script_program = Some(tx_script.code().to_bytes(AstSerdeOptions {
            serialize_imports: true,
        }));
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
        false,
        0_u32,
    ))
}

pub fn parse_transaction_columns(
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
    let committed: bool = row.get(10)?;
    let commit_height: u32 = row.get(11)?;

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
        committed,
        commit_height,
    ))
}

/// Parse a transaction from the provided parts.
fn parse_transaction(
    serialized_transaction: SerializedTransactionData,
) -> Result<TransactionStub, StoreError> {
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
        committed,
        commit_height,
    ) = serialized_transaction;
    let account_id = AccountId::try_from(account_id as u64).map_err(StoreError::AccountError)?;
    let id: Digest = id.try_into().map_err(StoreError::HexParseError)?;
    let init_account_state: Digest = init_account_state
        .try_into()
        .map_err(StoreError::HexParseError)?;

    let final_account_state: Digest = final_account_state
        .try_into()
        .map_err(StoreError::HexParseError)?;

    let input_note_nullifiers: Vec<Digest> =
        serde_json::from_str(&input_notes).map_err(StoreError::JsonDataDeserializationError)?;

    let output_notes = OutputNotes::<NoteEnvelope>::read_from_bytes(&output_notes)
        .map_err(StoreError::DataDeserializationError)?;

    let transaction_script: Option<TransactionScript> = if script_hash.is_some() {
        let script_hash = script_hash
            .map(|hash| Digest::read_from_bytes(&hash))
            .transpose()
            .map_err(StoreError::DataDeserializationError)?
            .expect("Script hash should be included in the row");

        let script_program = script_program
            .map(|program| ProgramAst::from_bytes(&program))
            .transpose()
            .map_err(StoreError::DataDeserializationError)?
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
        )
        .map_err(StoreError::TransactionScriptError)?;
        Some(tx_script)
    } else {
        None
    };

    Ok(TransactionStub {
        id,
        account_id,
        init_account_state,
        final_account_state,
        input_note_nullifiers,
        output_notes,
        transaction_script,
        block_num,
        committed,
        commit_height: commit_height as u64,
    })
}

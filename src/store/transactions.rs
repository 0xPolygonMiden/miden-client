use crate::{client::transactions::TransactionStub, errors::StoreError};
use crypto::{
    utils::{collections::BTreeMap, Deserializable, Serializable},
    Felt,
};

use objects::{
    accounts::AccountId,
    assembly::{AstSerdeOptions, ProgramAst},
    notes::{NoteEnvelope, NoteId},
    transaction::{ExecutedTransaction, OutputNotes, ProvenTransaction, TransactionScript},
    Digest,
};
use rusqlite::{params, Transaction};

use super::{
    notes::{serialize_input_note, InputNoteRecord, INSERT_NOTE_QUERY},
    Store,
};

pub(crate) const INSERT_TRANSACTION_QUERY: &str = "INSERT INTO transactions (id, account_id, init_account_state, final_account_state, \
    input_notes, output_notes, script_hash, script_program, script_inputs, block_num, committed, commit_height) \
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

// TRANSACTIONS FILTERS
// ================================================================================================

pub enum TransactionFilter {
    All,
    Uncomitted,
}

impl TransactionFilter {
    pub fn to_query(&self) -> String {
        const QUERY: &str = "SELECT id, account_id, init_account_state, final_account_state, \
        input_notes, output_notes, script_hash, script_program, script_inputs, block_num, committed, commit_height FROM transactions";
        match self {
            TransactionFilter::All => QUERY.to_string(),
            TransactionFilter::Uncomitted => format!("{QUERY} WHERE committed=false"),
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

    // TODO: Make a separate table for transaction scripts and only save the root
    pub fn insert_transaction(
        &self,
        transaction: &ProvenTransaction,
        tx_script: Option<TransactionScript>,
    ) -> Result<(), StoreError> {
        let (
            transaction_id,
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
        ) = serialize_transaction(transaction, tx_script)?;

        self.db
            .execute(
                INSERT_TRANSACTION_QUERY,
                params![
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
                ],
            )
            .map(|_| ())
            .map_err(StoreError::QueryError)
    }

    pub fn insert_proven_transaction_data(
        &mut self,
        proven_transaction: ProvenTransaction,
        transaction_result: ExecutedTransaction,
    ) -> Result<(), StoreError> {
        // Create atomic transcation

        let tx = self
            .db
            .transaction()
            .map_err(StoreError::TransactionError)?;

        // Insert transaction data

        let (
            transaction_id,
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
        ) = serialize_transaction(&proven_transaction, transaction_result.tx_script().cloned())?;

        tx.execute(
            INSERT_TRANSACTION_QUERY,
            params![
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
            ],
        )
        .map(|_| ())
        .map_err(StoreError::QueryError)?;

        let input_notes: Vec<InputNoteRecord> = transaction_result
            .input_notes()
            .iter()
            .map(|n| n.note().clone().into())
            .collect();

        // Insert input notes
        insert_input_notes(&tx, &input_notes)?;

        // commit the transaction
        tx.commit().map_err(StoreError::QueryError)?;

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

pub(crate) fn serialize_transaction(
    transaction: &ProvenTransaction,
    tx_script: Option<TransactionScript>,
) -> Result<SerializedTransactionData, StoreError> {
    let transaction_id: String = transaction.id().inner().into();
    let account_id: u64 = transaction.account_id().into();
    let init_account_state = &transaction.initial_account_hash().to_string();
    let final_account_state = &transaction.final_account_hash().to_string();

    // TODO: Double check if saving nullifiers as input notes is enough
    let nullifiers: Vec<Digest> = transaction
        .input_notes()
        .iter()
        .map(|x| x.inner())
        .collect();

    let input_notes =
        serde_json::to_string(&nullifiers).map_err(StoreError::InputSerializationError)?;

    let output_notes = transaction.output_notes();

    // TODO: Add proper logging
    println!("transaction id {:?}", transaction.id());
    println!("transaction account id: {}", transaction.account_id());
    println!("transaction output notes {:?}", output_notes);

    // TODO: Scripts should be in their own tables and only identifiers should be stored here
    let mut script_program = None;
    let mut script_hash = None;
    let mut script_inputs = None;

    if let Some(tx_script) = tx_script {
        script_program = Some(tx_script.code().to_bytes(AstSerdeOptions {
            serialize_imports: true,
        }));
        script_hash = Some(tx_script.hash().to_bytes());
        script_inputs = Some(
            serde_json::to_string(&tx_script.inputs())
                .map_err(StoreError::InputSerializationError)?,
        );
    }

    let block_num = 0u32;

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
        block_num,
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

    let output_notes: OutputNotes<NoteEnvelope> =
        OutputNotes::<NoteEnvelope>::read_from_bytes(&output_notes)
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

/// Inserts the provided input notes into the database
fn insert_input_notes(
    sql_transaction: &Transaction<'_>,
    notes: &[InputNoteRecord],
) -> Result<(), StoreError> {
    for note in notes {
        let (
            note_id,
            nullifier,
            script,
            vault,
            inputs,
            serial_num,
            sender_id,
            tag,
            num_assets,
            inclusion_proof,
            recipients,
            status,
            commit_height,
        ) = serialize_input_note(note)?;

        sql_transaction
            .execute(
                INSERT_NOTE_QUERY,
                params![
                    note_id,
                    nullifier,
                    script,
                    vault,
                    inputs,
                    serial_num,
                    sender_id,
                    tag,
                    num_assets,
                    inclusion_proof,
                    recipients,
                    status,
                    commit_height
                ],
            )
            .map_err(StoreError::QueryError)
            .map(|_| ())?
    }
    Ok(())
}

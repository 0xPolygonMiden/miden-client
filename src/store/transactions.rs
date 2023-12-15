use crypto::{
    utils::{collections::BTreeMap, Deserializable, Serializable},
    Felt,
};

use objects::{
    accounts::AccountId,
    assembly::{AstSerdeOptions, ProgramAst},
    notes::Note,
    transaction::{ProvenTransaction, TransactionScript},
    Digest,
};
use rusqlite::params;

use crate::{client::transactions::TransactionStub, errors::StoreError};

use super::Store;

// TRANSACTIONS
// ================================================================================================

type SerializedTransactionData = (
    String,
    i64,
    String,
    String,
    String,
    String,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
    Option<String>,
    u32,
    bool,
    u32,
);

impl Store {
    /// Retrieves all executed transactions from the database
    pub fn get_transactions(&self) -> Result<Vec<TransactionStub>, StoreError> {
        self
                .db
                .prepare("SELECT id, account_id, init_account_state, final_account_state, \
                input_notes, output_notes, script_hash, script_program, script_inputs, block_num, committed, commit_height FROM transactions")
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
            script_program,
            script_hash,
            script_inputs,
            block_ref,
            committed,
            commit_height,
        ) = serialize_transaction(transaction, tx_script)?;

        self.db.execute(
                "INSERT INTO transactions (id, account_id, init_account_state, final_account_state, \
                input_notes, output_notes, script_hash, script_program, script_inputs, block_ref, committed, commit_height) \
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    transaction_id,
                    { account_id },
                    init_account_state,
                    final_account_state,
                    input_notes,
                    output_notes,
                    script_program,
                    script_hash,
                    script_inputs,
                    block_ref,
                    committed,
                    commit_height,
                ],
            ).map(|_| ())
            .map_err(StoreError::QueryError)
    }
}

pub fn serialize_transaction(
    transaction: &ProvenTransaction,
    tx_script: Option<TransactionScript>,
) -> Result<SerializedTransactionData, StoreError> {
    let account_id: u64 = transaction.account_id().into();
    let init_account_state = &transaction.initial_account_hash().to_string();
    let final_account_state = &transaction.final_account_hash().to_string();

    // TODO: Double check if saving nullifiers as input notes is enough
    let nullifiers: Vec<Digest> = transaction
        .consumed_notes()
        .iter()
        .map(|x| x.inner())
        .collect();

    let input_notes =
        serde_json::to_string(&nullifiers).map_err(StoreError::InputSerializationError)?;

    let output_notes = serde_json::to_string(&transaction.created_notes().to_vec())
        .map_err(StoreError::InputSerializationError)?;

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
        transaction.final_account_hash().to_string(),
        account_id as i64,
        init_account_state.to_owned(),
        final_account_state.to_owned(),
        input_notes,
        output_notes,
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
    let output_notes: String = row.get(5)?;
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
    let output_notes: Vec<Note> =
        serde_json::from_str(&output_notes).map_err(StoreError::JsonDataDeserializationError)?;

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

use alloc::collections::{BTreeMap, BTreeSet};

use miden_lib::notes::{create_p2id_note, create_p2idr_note};
use miden_objects::{
    accounts::{AccountDelta, AccountId},
    assembly::ProgramAst,
    assets::FungibleAsset,
    crypto::rand::RpoRandomCoin,
    notes::{Note, NoteId, NoteType},
    transaction::{
        ExecutedTransaction, OutputNotes, ProvenTransaction, TransactionArgs, TransactionId,
        TransactionScript,
    },
    Digest, Felt, Word,
};
use miden_tx::{ProvingOptions, ScriptTarget, TransactionProver};
use rand::Rng;
use tracing::info;

use self::transaction_request::{PaymentTransactionData, TransactionRequest, TransactionTemplate};
use super::{note_screener::NoteRelevance, rpc::NodeRpcClient, Client, FeltRng};
use crate::{
    client::NoteScreener,
    errors::ClientError,
    store::{AuthInfo, Store, TransactionFilter},
};

pub mod transaction_request;

// TRANSACTION RESULT
// --------------------------------------------------------------------------------------------

/// Represents the result of executing a transaction by the client
///  
/// It contains an [ExecutedTransaction], a list of [Note] that describe the details of the notes
/// created by the transaction execution, and a list of `usize` `relevant_notes` that contain the
/// indices of `output_notes` that are relevant to the client
pub struct TransactionResult {
    executed_transaction: ExecutedTransaction,
    output_notes: Vec<Note>,
    relevant_notes: Option<BTreeMap<usize, Vec<(AccountId, NoteRelevance)>>>,
}

impl TransactionResult {
    pub fn new(
        executed_transaction: ExecutedTransaction,
        created_notes: Vec<Note>,
    ) -> Self {
        Self {
            executed_transaction,
            output_notes: created_notes,
            relevant_notes: None,
        }
    }

    pub fn executed_transaction(&self) -> &ExecutedTransaction {
        &self.executed_transaction
    }

    pub fn created_notes(&self) -> &Vec<Note> {
        &self.output_notes
    }

    pub fn relevant_notes(&self) -> Vec<&Note> {
        if let Some(relevant_notes) = &self.relevant_notes {
            relevant_notes
                .keys()
                .map(|note_index| &self.output_notes[*note_index])
                .collect()
        } else {
            self.created_notes().iter().collect()
        }
    }

    pub fn set_relevant_notes(
        &mut self,
        relevant_notes: BTreeMap<usize, Vec<(AccountId, NoteRelevance)>>,
    ) {
        self.relevant_notes = Some(relevant_notes);
    }

    pub fn block_num(&self) -> u32 {
        self.executed_transaction.block_header().block_num()
    }

    pub fn transaction_arguments(&self) -> &TransactionArgs {
        self.executed_transaction.tx_args()
    }

    pub fn account_delta(&self) -> &AccountDelta {
        self.executed_transaction.account_delta()
    }
}

// TRANSACTION RECORD
// --------------------------------------------------------------------------------------------

/// Describes a transaction that has been executed and is being tracked on the Client
///
/// Currently, the `commit_height` (and `committed` status) is set based on the height
/// at which the transaction's output notes are committed.
pub struct TransactionRecord {
    pub id: TransactionId,
    pub account_id: AccountId,
    pub init_account_state: Digest,
    pub final_account_state: Digest,
    pub input_note_nullifiers: Vec<Digest>,
    pub output_notes: OutputNotes,
    pub transaction_script: Option<TransactionScript>,
    pub block_num: u32,
    pub transaction_status: TransactionStatus,
}

impl TransactionRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: TransactionId,
        account_id: AccountId,
        init_account_state: Digest,
        final_account_state: Digest,
        input_note_nullifiers: Vec<Digest>,
        output_notes: OutputNotes,
        transaction_script: Option<TransactionScript>,
        block_num: u32,
        transaction_status: TransactionStatus,
    ) -> TransactionRecord {
        TransactionRecord {
            id,
            account_id,
            init_account_state,
            final_account_state,
            input_note_nullifiers,
            output_notes,
            transaction_script,
            block_num,
            transaction_status,
        }
    }
}

/// Represents the status of a transaction
pub enum TransactionStatus {
    /// Transaction has been submitted but not yet committed
    Pending,
    /// Transaction has been committed and included at the specified block number
    Committed(u32),
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            TransactionStatus::Pending => write!(f, "Pending"),
            TransactionStatus::Committed(block_number) => {
                write!(f, "Committed (Block: {})", block_number)
            },
        }
    }
}

impl<N: NodeRpcClient, R: FeltRng, S: Store> Client<N, R, S> {
    // TRANSACTION DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Retrieves tracked transactions, filtered by [TransactionFilter].
    pub fn get_transactions(
        &self,
        filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, ClientError> {
        self.store.get_transactions(filter).map_err(|err| err.into())
    }

    // TRANSACTION
    // --------------------------------------------------------------------------------------------

    /// Compiles a [TransactionTemplate] into a [TransactionRequest] that can be then executed by the
    /// client
    pub fn build_transaction_request(
        &mut self,
        transaction_template: TransactionTemplate,
    ) -> Result<TransactionRequest, ClientError> {
        let account_id = transaction_template.account_id();
        let account_auth = self.store.get_account_auth(account_id)?;

        match transaction_template {
            TransactionTemplate::ConsumeNotes(_, notes) => {
                let program_ast = ProgramAst::parse(transaction_request::AUTH_CONSUME_NOTES_SCRIPT)
                    .expect("shipped MASM is well-formed");
                let notes = notes.iter().map(|id| (*id, None)).collect();

                let tx_script = {
                    let script_inputs = vec![account_auth.into_advice_inputs()];
                    self.tx_executor.compile_tx_script(program_ast, script_inputs, vec![])?
                };
                Ok(TransactionRequest::new(account_id, notes, vec![], Some(tx_script)))
            },
            TransactionTemplate::MintFungibleAsset(asset, target_account_id) => {
                self.build_mint_tx_request(asset, account_auth, target_account_id)
            },
            TransactionTemplate::PayToId(payment_data) => {
                self.build_p2id_tx_request(account_auth, payment_data, None)
            },
            TransactionTemplate::PayToIdWithRecall(payment_data, recall_height) => {
                self.build_p2id_tx_request(account_auth, payment_data, Some(recall_height))
            },
        }
    }

    /// Creates and executes a transaction specified by the template, but does not change the
    /// local database.
    ///
    /// # Errors
    ///
    /// - Returns [ClientError::OutputNotesDoNotMatch] if the [TransactionRequest] ouput notes do
    /// not match the executor's output notes
    /// - Returns a [ClientError::TransactionExecutionError]
    pub fn new_transaction(
        &mut self,
        transaction_request: TransactionRequest,
    ) -> Result<TransactionResult, ClientError> {
        let account_id = transaction_request.account_id();
        self.tx_executor
            .load_account(account_id)
            .map_err(ClientError::TransactionExecutionError)?;

        let block_num = self.store.get_sync_height()?;

        let note_ids = transaction_request.get_input_note_ids();

        let output_notes = transaction_request.expected_output_notes().to_vec();

        // Execute the transaction and get the witness
        let executed_transaction = self.tx_executor.execute_transaction(
            account_id,
            block_num,
            &note_ids,
            transaction_request.into(),
        )?;

        // Check that the expected output notes is a subset of the transaction's output notes
        let tx_note_ids: BTreeSet<NoteId> =
            executed_transaction.output_notes().iter().map(|n| n.id()).collect();

        let missing_note_ids: Vec<NoteId> = output_notes
            .iter()
            .filter_map(|n| (!tx_note_ids.contains(&n.id())).then_some(n.id()))
            .collect();

        if !missing_note_ids.is_empty() {
            return Err(ClientError::MissingOutputNotes(missing_note_ids));
        }

        Ok(TransactionResult::new(executed_transaction, output_notes))
    }

    /// Proves the specified transaction witness, submits it to the node, and stores the transaction in
    /// the local database for tracking.
    pub async fn submit_transaction(
        &mut self,
        tx_result: TransactionResult,
    ) -> Result<(), ClientError> {
        let transaction_prover = TransactionProver::new(ProvingOptions::default());
        let proven_transaction =
            transaction_prover.prove_transaction(tx_result.executed_transaction().clone())?;

        info!("Proved transaction, submitting to the node...");

        self.submit_proven_transaction_request(proven_transaction.clone()).await?;

        let note_screener = NoteScreener::new(&self.store);
        let mut relevant_notes = BTreeMap::new();

        for (idx, note) in tx_result.created_notes().iter().enumerate() {
            let account_relevance = note_screener.check_relevance(note)?;
            if !account_relevance.is_empty() {
                relevant_notes.insert(idx, account_relevance);
            }
        }

        let mut tx_result = tx_result;
        tx_result.set_relevant_notes(relevant_notes);

        // Transaction was proven and submitted to the node correctly, persist note details and update account
        self.store.apply_transaction(tx_result)?;

        Ok(())
    }

    /// Compiles the provided transaction script source and inputs into a [TransactionScript] and
    /// checks (to the extent possible) that the transaction script can be executed against all
    /// accounts with the specified interfaces.
    pub fn compile_tx_script<T>(
        &self,
        program: ProgramAst,
        inputs: T,
        target_account_procs: Vec<ScriptTarget>,
    ) -> Result<TransactionScript, ClientError>
    where
        T: IntoIterator<Item = (Word, Vec<Felt>)>,
    {
        self.tx_executor
            .compile_tx_script(program, inputs, target_account_procs)
            .map_err(ClientError::TransactionExecutionError)
    }

    async fn submit_proven_transaction_request(
        &mut self,
        proven_transaction: ProvenTransaction,
    ) -> Result<(), ClientError> {
        Ok(self.rpc_api.submit_proven_transaction(proven_transaction).await?)
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Gets [RpoRandomCoin] from the client
    fn get_random_coin(&self) -> RpoRandomCoin {
        // TODO: Initialize coin status once along with the client and persist status for retrieval
        let mut rng = rand::thread_rng();
        let coin_seed: [u64; 4] = rng.gen();

        RpoRandomCoin::new(coin_seed.map(Felt::new))
    }

    /// Helper to build a [TransactionRequest] for P2ID-type transactions easily.
    ///
    /// - auth_info has to be from the executor account
    /// - If recall_height is Some(), a P2IDR note will be created. Otherwise, a P2ID is created.
    fn build_p2id_tx_request(
        &self,
        auth_info: AuthInfo,
        payment_data: PaymentTransactionData,
        recall_height: Option<u32>,
    ) -> Result<TransactionRequest, ClientError> {
        let random_coin = self.get_random_coin();

        let created_note = if let Some(recall_height) = recall_height {
            create_p2idr_note(
                payment_data.account_id(),
                payment_data.target_account_id(),
                vec![payment_data.asset()],
                NoteType::OffChain, // TODO: Select correct note type
                recall_height,
                random_coin,
            )?
        } else {
            create_p2id_note(
                payment_data.account_id(),
                payment_data.target_account_id(),
                vec![payment_data.asset()],
                NoteType::OffChain, // TODO: Select correct note type
                random_coin,
            )?
        };

        let recipient = created_note
            .recipient_digest()
            .iter()
            .map(|x| x.as_int().to_string())
            .collect::<Vec<_>>()
            .join(".");

        let note_tag = created_note.metadata().tag().inner();

        let tx_script = ProgramAst::parse(
            &transaction_request::AUTH_SEND_ASSET_SCRIPT
                .replace("{recipient}", &recipient)
                .replace("{note_type}", &Felt::new(NoteType::OffChain as u64).to_string())
                .replace("{tag}", &Felt::new(note_tag.into()).to_string())
                .replace("{asset}", &prepare_word(&payment_data.asset().into()).to_string()),
        )
        .expect("shipped MASM is well-formed");

        let tx_script = {
            let script_inputs = vec![auth_info.into_advice_inputs()];
            self.tx_executor.compile_tx_script(tx_script, script_inputs, vec![])?
        };

        Ok(TransactionRequest::new(
            payment_data.account_id(),
            BTreeMap::new(),
            vec![created_note],
            Some(tx_script),
        ))
    }

    /// Helper to build a [TransactionRequest] for transaction to mint fungible tokens.
    ///
    /// - faucet_auth_info has to be from the faucet account
    fn build_mint_tx_request(
        &self,
        asset: FungibleAsset,
        faucet_auth_info: AuthInfo,
        target_account_id: AccountId,
    ) -> Result<TransactionRequest, ClientError> {
        let random_coin = self.get_random_coin();
        let created_note = create_p2id_note(
            asset.faucet_id(),
            target_account_id,
            vec![asset.into()],
            NoteType::OffChain, // TODO: Select correct note type
            random_coin,
        )?;

        let recipient = created_note
            .recipient_digest()
            .iter()
            .map(|x| x.as_int().to_string())
            .collect::<Vec<_>>()
            .join(".");

        let note_tag = created_note.metadata().tag().inner();

        let tx_script = ProgramAst::parse(
            &transaction_request::DISTRIBUTE_FUNGIBLE_ASSET_SCRIPT
                .replace("{recipient}", &recipient)
                .replace("{note_type}", &Felt::new(NoteType::OffChain as u64).to_string())
                .replace("{tag}", &Felt::new(note_tag.into()).to_string())
                .replace("{amount}", &Felt::new(asset.amount()).to_string()),
        )
        .expect("shipped MASM is well-formed");

        let tx_script = {
            let script_inputs = vec![faucet_auth_info.into_advice_inputs()];
            self.tx_executor.compile_tx_script(tx_script, script_inputs, vec![])?
        };

        Ok(TransactionRequest::new(
            asset.faucet_id(),
            BTreeMap::new(),
            vec![created_note],
            Some(tx_script),
        ))
    }
}

// HELPERS
// ================================================================================================

pub(crate) fn prepare_word(word: &Word) -> String {
    word.iter().map(|x| x.as_int().to_string()).collect::<Vec<_>>().join(".")
}

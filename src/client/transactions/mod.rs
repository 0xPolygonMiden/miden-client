use alloc::collections::{BTreeMap, BTreeSet};

use miden_lib::notes::{create_p2id_note, create_p2idr_note, create_swap_note};
use miden_objects::{
    accounts::{AccountDelta, AccountId, AuthSecretKey},
    assembly::ProgramAst,
    assets::FungibleAsset,
    crypto::rand::RpoRandomCoin,
    notes::{Note, NoteDetails, NoteId, NoteType},
    transaction::{
        ExecutedTransaction, InputNotes, OutputNote, OutputNotes, ProvenTransaction,
        TransactionArgs, TransactionId, TransactionScript,
    },
    Digest, Felt, Word,
};
use miden_tx::{ProvingOptions, ScriptTarget, TransactionAuthenticator, TransactionProver};
use rand::Rng;
use tracing::info;

use self::transaction_request::{
    PaymentTransactionData, SwapTransactionData, TransactionRequest, TransactionTemplate,
};
use super::{rpc::NodeRpcClient, Client, FeltRng};
use crate::{
    client::NoteScreener,
    errors::ClientError,
    store::{InputNoteRecord, Store, TransactionFilter},
};

pub mod transaction_request;

// TRANSACTION RESULT
// --------------------------------------------------------------------------------------------

/// Represents the result of executing a transaction by the client.
///  
/// It contains an [ExecutedTransaction], and a list of `relevant_notes` that contains the
/// `output_notes` that the client has to store as input notes, based on the NoteScreener
/// output from filtering the transaction's output notes or some partial note we expect to receive
/// in the future (you can check at swap notes for an example of this).
pub struct TransactionResult {
    transaction: ExecutedTransaction,
    relevant_notes: Vec<InputNoteRecord>,
}

impl TransactionResult {
    /// Screens the output notes to store and track the relevant ones, and instantiates a [TransactionResult]
    pub fn new<S: Store>(
        transaction: ExecutedTransaction,
        note_screener: NoteScreener<S>,
        partial_notes: Vec<NoteDetails>,
    ) -> Result<Self, ClientError> {
        let mut relevant_notes = vec![];

        for note in notes_from_output(transaction.output_notes()) {
            let account_relevance = note_screener.check_relevance(note)?;

            if !account_relevance.is_empty() {
                relevant_notes.push(note.clone().into());
            }
        }

        // Include partial output notes into the relevant notes
        relevant_notes.extend(partial_notes.iter().map(InputNoteRecord::from));

        let tx_result = Self { transaction, relevant_notes };

        Ok(tx_result)
    }

    pub fn executed_transaction(&self) -> &ExecutedTransaction {
        &self.transaction
    }

    pub fn created_notes(&self) -> &OutputNotes {
        self.transaction.output_notes()
    }

    pub fn relevant_notes(&self) -> &[InputNoteRecord] {
        &self.relevant_notes
    }

    pub fn block_num(&self) -> u32 {
        self.transaction.block_header().block_num()
    }

    pub fn transaction_arguments(&self) -> &TransactionArgs {
        self.transaction.tx_args()
    }

    pub fn account_delta(&self) -> &AccountDelta {
        self.transaction.account_delta()
    }

    pub fn consumed_notes(&self) -> &InputNotes {
        self.transaction.tx_inputs().input_notes()
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionStatus::Pending => write!(f, "Pending"),
            TransactionStatus::Committed(block_number) => {
                write!(f, "Committed (Block: {})", block_number)
            },
        }
    }
}

impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> Client<N, R, S, A> {
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

        let (_pk, _sk) = match account_auth {
            AuthSecretKey::RpoFalcon512(key) => {
                (key.public_key(), AuthSecretKey::RpoFalcon512(key))
            },
        };

        match transaction_template {
            TransactionTemplate::ConsumeNotes(_, notes) => {
                let program_ast = ProgramAst::parse(transaction_request::AUTH_CONSUME_NOTES_SCRIPT)
                    .expect("shipped MASM is well-formed");
                let notes = notes.iter().map(|id| (*id, None)).collect();

                let tx_script = self.tx_executor.compile_tx_script(program_ast, vec![], vec![])?;
                Ok(TransactionRequest::new(account_id, notes, vec![], vec![], Some(tx_script)))
            },
            TransactionTemplate::MintFungibleAsset(asset, target_account_id, note_type) => {
                self.build_mint_tx_request(asset, target_account_id, note_type)
            },
            TransactionTemplate::PayToId(payment_data, note_type) => {
                self.build_p2id_tx_request(payment_data, None, note_type)
            },
            TransactionTemplate::PayToIdWithRecall(payment_data, recall_height, note_type) => {
                self.build_p2id_tx_request(payment_data, Some(recall_height), note_type)
            },
            TransactionTemplate::Swap(swap_data, note_type) => {
                self.build_swap_tx_request(swap_data, note_type)
            },
        }
    }

    /// Creates and executes a transaction specified by the template, but does not change the
    /// local database.
    ///
    /// # Errors
    ///
    /// - Returns [ClientError::MissingOutputNotes] if the [TransactionRequest] ouput notes are
    ///   not a subset of executor's output notes
    /// - Returns a [ClientError::TransactionExecutorError] if the execution fails
    pub fn new_transaction(
        &mut self,
        transaction_request: TransactionRequest,
    ) -> Result<TransactionResult, ClientError> {
        let account_id = transaction_request.account_id();
        self.tx_executor
            .load_account(account_id)
            .map_err(ClientError::TransactionExecutorError)?;

        let block_num = self.store.get_sync_height()?;

        let note_ids = transaction_request.get_input_note_ids();
        let output_notes = transaction_request.expected_output_notes().to_vec();
        let partial_notes = transaction_request.expected_partial_notes().to_vec();

        // Execute the transaction and get the witness
        let executed_transaction = self.tx_executor.execute_transaction(
            account_id,
            block_num,
            &note_ids,
            transaction_request.into(),
        )?;

        // Check that the expected output notes matches the transaction outcome.
        // We comprare authentication hashes where possible since that involves note IDs + metadata
        // (as opposed to just note ID which remains the same regardless of metadata)
        // We also do the check for partial output notes
        let tx_note_auth_hashes: BTreeSet<Digest> =
            notes_from_output(executed_transaction.output_notes())
                .map(Note::authentication_hash)
                .collect();

        let missing_note_ids: Vec<NoteId> = output_notes
            .iter()
            .filter_map(|n| {
                (!tx_note_auth_hashes.contains(&n.authentication_hash())).then_some(n.id())
            })
            .collect();

        if !missing_note_ids.is_empty() {
            return Err(ClientError::MissingOutputNotes(missing_note_ids));
        }

        let screener = NoteScreener::new(self.store.clone());

        TransactionResult::new(executed_transaction, screener, partial_notes)
    }

    /// Proves the specified transaction witness, and returns a [ProvenTransaction] that can be
    /// submitted to the node.
    pub fn prove_transaction(
        &mut self,
        executed_transaction: ExecutedTransaction,
    ) -> Result<ProvenTransaction, ClientError> {
        let transaction_prover = TransactionProver::new(ProvingOptions::default());

        let proven_transaction = transaction_prover.prove_transaction(executed_transaction)?;
        Ok(proven_transaction)
    }

    /// Submits a [ProvenTransaction] to the node, and stores the transaction in
    /// the local database for tracking.
    pub async fn submit_transaction(
        &mut self,
        tx_result: TransactionResult,
        proven_transaction: ProvenTransaction,
    ) -> Result<(), ClientError> {
        self.rpc_api.submit_proven_transaction(proven_transaction).await?;
        info!("Transaction submitted");

        // Transaction was proven and submitted to the node correctly, persist note details and update account
        self.store.apply_transaction(tx_result)?;
        info!("Transaction stored");
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
            .map_err(ClientError::TransactionExecutorError)
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
        payment_data: PaymentTransactionData,
        recall_height: Option<u32>,
        note_type: NoteType,
    ) -> Result<TransactionRequest, ClientError> {
        let random_coin = self.get_random_coin();

        let created_note = if let Some(recall_height) = recall_height {
            create_p2idr_note(
                payment_data.account_id(),
                payment_data.target_account_id(),
                vec![payment_data.asset()],
                note_type,
                recall_height,
                random_coin,
            )?
        } else {
            create_p2id_note(
                payment_data.account_id(),
                payment_data.target_account_id(),
                vec![payment_data.asset()],
                note_type,
                random_coin,
            )?
        };

        let recipient = created_note
            .recipient()
            .digest()
            .iter()
            .map(|x| x.as_int().to_string())
            .collect::<Vec<_>>()
            .join(".");

        let note_tag = created_note.metadata().tag().inner();

        let tx_script = ProgramAst::parse(
            &transaction_request::AUTH_SEND_ASSET_SCRIPT
                .replace("{recipient}", &recipient)
                .replace("{note_type}", &Felt::new(note_type as u64).to_string())
                .replace("{tag}", &Felt::new(note_tag.into()).to_string())
                .replace("{asset}", &prepare_word(&payment_data.asset().into()).to_string()),
        )
        .expect("shipped MASM is well-formed");

        let tx_script = self.tx_executor.compile_tx_script(tx_script, vec![], vec![])?;

        Ok(TransactionRequest::new(
            payment_data.account_id(),
            BTreeMap::new(),
            vec![created_note],
            vec![],
            Some(tx_script),
        ))
    }

    /// Helper to build a [TransactionRequest] for Swap-type transactions easily.
    ///
    /// - auth_info has to be from the executor account
    fn build_swap_tx_request(
        &self,
        swap_data: SwapTransactionData,
        note_type: NoteType,
    ) -> Result<TransactionRequest, ClientError> {
        let random_coin = self.get_random_coin();

        // The created note is the one that we need as the output of the tx, the other one is the
        // one that we expect to receive and consume eventually
        let (created_note, payback_note_details) = create_swap_note(
            swap_data.account_id(),
            swap_data.offered_asset(),
            swap_data.requested_asset(),
            note_type,
            random_coin,
        )?;

        let recipient = created_note
            .recipient()
            .digest()
            .iter()
            .map(|x| x.as_int().to_string())
            .collect::<Vec<_>>()
            .join(".");

        let note_tag = created_note.metadata().tag().inner();

        let tx_script = ProgramAst::parse(
            &transaction_request::AUTH_SEND_ASSET_SCRIPT
                .replace("{recipient}", &recipient)
                .replace("{note_type}", &Felt::new(note_type as u64).to_string())
                .replace("{tag}", &Felt::new(note_tag.into()).to_string())
                .replace("{asset}", &prepare_word(&swap_data.offered_asset().into()).to_string()),
        )
        .expect("shipped MASM is well-formed");

        let tx_script = self.tx_executor.compile_tx_script(tx_script, vec![], vec![])?;

        Ok(TransactionRequest::new(
            swap_data.account_id(),
            BTreeMap::new(),
            vec![created_note],
            vec![payback_note_details],
            Some(tx_script),
        ))
    }

    /// Helper to build a [TransactionRequest] for transaction to mint fungible tokens.
    ///
    /// - faucet_auth_info has to be from the faucet account
    fn build_mint_tx_request(
        &self,
        asset: FungibleAsset,
        target_account_id: AccountId,
        note_type: NoteType,
    ) -> Result<TransactionRequest, ClientError> {
        let random_coin = self.get_random_coin();
        let created_note = create_p2id_note(
            asset.faucet_id(),
            target_account_id,
            vec![asset.into()],
            note_type,
            random_coin,
        )?;

        let recipient = created_note
            .recipient()
            .digest()
            .iter()
            .map(|x| x.as_int().to_string())
            .collect::<Vec<_>>()
            .join(".");

        let note_tag = created_note.metadata().tag().inner();

        let tx_script = ProgramAst::parse(
            &transaction_request::DISTRIBUTE_FUNGIBLE_ASSET_SCRIPT
                .replace("{recipient}", &recipient)
                .replace("{note_type}", &Felt::new(note_type as u64).to_string())
                .replace("{tag}", &Felt::new(note_tag.into()).to_string())
                .replace("{amount}", &Felt::new(asset.amount()).to_string()),
        )
        .expect("shipped MASM is well-formed");

        let tx_script = self.tx_executor.compile_tx_script(tx_script, vec![], vec![])?;

        Ok(TransactionRequest::new(
            asset.faucet_id(),
            BTreeMap::new(),
            vec![created_note],
            vec![],
            Some(tx_script),
        ))
    }
}

// HELPERS
// ================================================================================================

pub(crate) fn prepare_word(word: &Word) -> String {
    word.iter().map(|x| x.as_int().to_string()).collect::<Vec<_>>().join(".")
}

/// Extracts notes from [OutputNotes]
/// Used for:
/// - checking the relevance of notes to save them as input notes
/// - validate hashes versus expected output notes after a transaction is executed
pub(crate) fn notes_from_output(output_notes: &OutputNotes) -> impl Iterator<Item = &Note> {
    output_notes
        .iter()
        .filter(|n| matches!(n, OutputNote::Full(_)))
        .map(|n| match n {
            OutputNote::Full(n) => n,
            // The following todo!() applies until we have a way to support flows where we have
            // partial details of the note
            OutputNote::Partial(_) => {
                todo!("For now, all details should be held in OutputNote::Fulls")
            },
            OutputNote::Header(_) => {
                todo!("For now, all details should be held in OutputNote::Fulls")
            },
        })
}

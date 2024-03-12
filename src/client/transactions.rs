use crate::store::Store;
use crypto::{rand::RpoRandomCoin, utils::Serializable, Felt, Word};
use miden_lib::notes::create_p2id_note;
use miden_objects::{
    accounts::{AccountDelta, AccountId},
    assembly::ProgramAst,
    assets::{Asset, FungibleAsset},
    notes::{Note, NoteId},
    transaction::{
        ExecutedTransaction, OutputNote, OutputNotes, ProvenTransaction, TransactionArgs,
        TransactionId, TransactionScript,
    },
    Digest,
};
use miden_tx::{DataStore, ProvingOptions, TransactionProver};
use rand::Rng;
use tracing::info;

use crate::{
    errors::ClientError,
    store::{AuthInfo, TransactionFilter},
};

use super::{rpc::NodeRpcClient, Client};

// MASM SCRIPTS
// --------------------------------------------------------------------------------------------
const AUTH_CONSUME_NOTES_SCRIPT: &str =
    include_str!("asm/transaction_scripts/auth_consume_notes.masm");
const DISTRIBUTE_FUNGIBLE_ASSET_SCRIPT: &str =
    include_str!("asm/transaction_scripts/distribute_fungible_asset.masm");
const AUTH_SEND_ASSET_SCRIPT: &str = include_str!("asm/transaction_scripts/auth_send_asset.masm");

// TRANSACTION TEMPLATE
// --------------------------------------------------------------------------------------------

#[derive(Clone)]
pub enum TransactionTemplate {
    /// Consume outstanding notes for an account.
    ConsumeNotes(AccountId, Vec<NoteId>),
    /// Mint fungible assets using a faucet account
    MintFungibleAsset {
        asset: FungibleAsset,
        target_account_id: AccountId,
    },
    /// Creates a pay-to-id note directed to a specific account
    PayToId(PaymentTransactionData),
    /// Creates a pay-to-id note directed to a specific account, specifying a block height after
    /// which the note can be recalled
    PayToIdWithRecall(PaymentTransactionData, u32),
}

impl TransactionTemplate {
    /// Returns the executor [AccountId]
    pub fn account_id(&self) -> AccountId {
        match self {
            TransactionTemplate::ConsumeNotes(account_id, _) => *account_id,
            TransactionTemplate::MintFungibleAsset {
                asset,
                target_account_id: _target_account_id,
            } => asset.faucet_id(),
            TransactionTemplate::PayToId(p) => *p.account_id(),
            TransactionTemplate::PayToIdWithRecall(p, _) => *p.account_id(),
        }
    }
}

// PAYMENT TRANSACTION DATA
// --------------------------------------------------------------------------------------------

#[derive(Clone)]
pub struct PaymentTransactionData {
    asset: Asset,
    sender_account_id: AccountId,
    target_account_id: AccountId,
}

impl PaymentTransactionData {
    pub fn new(
        asset: Asset,
        sender_account_id: AccountId,
        target_account_id: AccountId,
    ) -> PaymentTransactionData {
        PaymentTransactionData {
            asset,
            sender_account_id,
            target_account_id,
        }
    }

    /// Returns the executor [AccountId]
    pub fn account_id(&self) -> &AccountId {
        &self.sender_account_id
    }
}

// TRANSACTION RESULT
// --------------------------------------------------------------------------------------------

/// Represents the result of executing a transaction by the client
///  
/// It contains an [ExecutedTransaction] and a list of [Note] that describe the details of the
/// notes created by the transaction execution
pub struct TransactionResult {
    executed_transaction: ExecutedTransaction,
    output_notes: Vec<Note>,
}

impl TransactionResult {
    pub fn new(executed_transaction: ExecutedTransaction, created_notes: Vec<Note>) -> Self {
        Self {
            executed_transaction,
            output_notes: created_notes,
        }
    }

    pub fn executed_transaction(&self) -> &ExecutedTransaction {
        &self.executed_transaction
    }

    pub fn created_notes(&self) -> &Vec<Note> {
        &self.output_notes
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
    pub output_notes: OutputNotes<OutputNote>,
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
        output_notes: OutputNotes<OutputNote>,
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
            }
        }
    }
}

impl<N: NodeRpcClient, S: Store, D: DataStore> Client<N, S, D> {
    // TRANSACTION DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Retrieves tracked transactions, filtered by [TransactionFilter].
    pub fn get_transactions(
        &self,
        filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, ClientError> {
        self.store
            .get_transactions(filter)
            .map_err(|err| err.into())
    }

    // TRANSACTION
    // --------------------------------------------------------------------------------------------

    /// Creates and executes a transaction specified by the template, but does not change the
    /// local database.
    pub fn new_transaction(
        &mut self,
        transaction_template: TransactionTemplate,
    ) -> Result<TransactionResult, ClientError> {
        match transaction_template {
            TransactionTemplate::PayToId(PaymentTransactionData {
                asset: fungible_asset,
                sender_account_id,
                target_account_id,
            }) => self.new_p2id_transaction(fungible_asset, sender_account_id, target_account_id),
            TransactionTemplate::PayToIdWithRecall(_payment_data, _recall_height) => todo!(),
            TransactionTemplate::ConsumeNotes(account_id, list_of_notes) => {
                self.new_consume_notes_transaction(account_id, &list_of_notes)
            }
            TransactionTemplate::MintFungibleAsset {
                asset,
                target_account_id,
            } => self.new_mint_fungible_asset_transaction(asset, target_account_id),
        }
    }

    /// Creates and executes a transaction that consumes a number of notes
    fn new_consume_notes_transaction(
        &mut self,
        account_id: AccountId,
        note_ids: &[NoteId],
    ) -> Result<TransactionResult, ClientError> {
        self.tx_executor
            .load_account(account_id)
            .map_err(ClientError::TransactionExecutionError)?;

        let tx_script_code =
            ProgramAst::parse(AUTH_CONSUME_NOTES_SCRIPT).expect("shipped MASM is well-formed");

        let block_num = self.store.get_sync_height()?;

        // Because the notes are retrieved by the executor, there is no need to cross check here
        // that they exist in the Store
        self.compile_and_execute_tx(account_id, note_ids, vec![], tx_script_code, block_num)
    }

    /// Creates and executes a mint transaction specified by the template.
    fn new_mint_fungible_asset_transaction(
        &mut self,
        asset: FungibleAsset,
        target_id: AccountId,
    ) -> Result<TransactionResult, ClientError> {
        let faucet_id = asset.faucet_id();

        // Construct Account
        self.tx_executor.load_account(faucet_id)?;

        let block_ref = self.get_sync_height()?;
        let random_coin = self.get_random_coin();

        let created_note = create_p2id_note(faucet_id, target_id, vec![asset.into()], random_coin)?;

        let recipient = created_note
            .recipient()
            .iter()
            .map(|x| x.as_int().to_string())
            .collect::<Vec<_>>()
            .join(".");

        let tx_script_code = ProgramAst::parse(
            &DISTRIBUTE_FUNGIBLE_ASSET_SCRIPT
                .replace("{recipient}", &recipient)
                .replace(
                    "{tag}",
                    &Felt::new(Into::<u64>::into(target_id)).to_string(),
                )
                .replace("{amount}", &Felt::new(asset.amount()).to_string()),
        )
        .expect("shipped MASM is well-formed");

        self.compile_and_execute_tx(
            faucet_id,
            &[],
            vec![created_note],
            tx_script_code,
            block_ref,
        )
    }

    fn new_p2id_transaction(
        &mut self,
        fungible_asset: Asset,
        sender_account_id: AccountId,
        target_account_id: AccountId,
    ) -> Result<TransactionResult, ClientError> {
        let random_coin = self.get_random_coin();

        let created_note = create_p2id_note(
            sender_account_id,
            target_account_id,
            vec![fungible_asset],
            random_coin,
        )?;

        self.tx_executor.load_account(sender_account_id)?;

        let block_ref = self.get_sync_height()?;

        let recipient = created_note
            .recipient()
            .iter()
            .map(|x| x.as_int().to_string())
            .collect::<Vec<_>>()
            .join(".");

        let tx_script_code = ProgramAst::parse(
            &AUTH_SEND_ASSET_SCRIPT
                .replace("{recipient}", &recipient)
                .replace(
                    "{tag}",
                    &Felt::new(Into::<u64>::into(target_account_id)).to_string(),
                )
                .replace("{asset}", &prepare_word(&fungible_asset.into()).to_string()),
        )
        .expect("shipped MASM is well-formed");

        self.compile_and_execute_tx(
            sender_account_id,
            &[],
            vec![created_note],
            tx_script_code,
            block_ref,
        )
    }

    fn compile_and_execute_tx(
        &mut self,
        account_id: AccountId,
        input_notes: &[NoteId],
        output_notes: Vec<Note>,
        tx_script: ProgramAst,
        block_num: u32,
    ) -> Result<TransactionResult, ClientError> {
        let account_auth = self.store.get_account_auth(account_id)?;
        let (pubkey_input, advice_map): (Word, Vec<Felt>) = match account_auth {
            AuthInfo::RpoFalcon512(key) => (
                key.public_key().into(),
                key.to_bytes()
                    .iter()
                    .map(|a| Felt::new(*a as u64))
                    .collect::<Vec<Felt>>(),
            ),
        };
        let script_inputs = vec![(pubkey_input, advice_map)];

        let tx_script = self
            .tx_executor
            .compile_tx_script(tx_script, script_inputs, vec![])?;

        let tx_args = TransactionArgs::with_tx_script(tx_script);

        // Execute the transaction and get the witness
        let executed_transaction = self.tx_executor.execute_transaction(
            account_id,
            block_num,
            input_notes,
            Some(tx_args),
        )?;

        Ok(TransactionResult::new(executed_transaction, output_notes))
    }

    /// Proves the specified transaction witness, submits it to the node, and stores the transaction in
    /// the local database for tracking.
    pub async fn send_transaction(
        &mut self,
        tx_result: TransactionResult,
    ) -> Result<(), ClientError> {
        let transaction_prover = TransactionProver::new(ProvingOptions::default());
        let proven_transaction =
            transaction_prover.prove_transaction(tx_result.executed_transaction().clone())?;

        info!("Proved transaction, submitting to the node...");

        self.submit_proven_transaction_request(proven_transaction.clone())
            .await?;

        // Transaction was proven and submitted to the node correctly, persist note details and update account
        self.store.apply_transaction(tx_result)?;

        Ok(())
    }

    async fn submit_proven_transaction_request(
        &mut self,
        proven_transaction: ProvenTransaction,
    ) -> Result<(), ClientError> {
        Ok(self
            .rpc_api
            .submit_proven_transaction(proven_transaction)
            .await?)
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
}

// HELPERS
// ================================================================================================

pub fn prepare_word(word: &Word) -> String {
    word.iter()
        .map(|x| x.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".")
}

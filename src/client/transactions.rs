use crypto::{rand::RpoRandomCoin, utils::Serializable, Felt, StarkField, Word};
use miden_lib::notes::create_p2id_note;
use miden_node_proto::{
    requests::SubmitProvenTransactionRequest, responses::SubmitProvenTransactionResponse,
};

use miden_tx::{ProvingOptions, TransactionProver};

use mock::procedures::prepare_word;
use objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    assets::{Asset, FungibleAsset},
    notes::{Note, NoteEnvelope, NoteId},
    transaction::{ExecutedTransaction, OutputNotes, ProvenTransaction, TransactionScript},
    Digest,
};
use rand::Rng;

use crate::{
    errors::ClientError,
    store::{accounts::AuthInfo, notes::InputNoteFilter, transactions::TransactionFilter},
};

use super::Client;

// TRANSACTION TEMPLATE
// --------------------------------------------------------------------------------------------

#[derive(Clone)]
pub enum TransactionTemplate {
    /// Consume outstanding note for an account. If none is provided, it consumes the first found
    ConsumeNote(AccountId, Option<NoteId>),
    // NOTE: Maybe this should be called "distribute"?
    /// Mint fungible assets using a faucet account
    MintFungibleAsset {
        asset: FungibleAsset,
        target_account_id: AccountId,
    },
    /// Creates a pay-to-id note directed to a specific account
    PayToId(PaymentTransactionData),
    /// Creates a pay-to-id note directed to a specific account, specifying a block height at which the payment is recalled
    PayToIdWithRecall(PaymentTransactionData, u32),
}

impl TransactionTemplate {
    /// Returns the executor [AccountId]
    pub fn account_id(&self) -> AccountId {
        match self {
            TransactionTemplate::ConsumeNote(account_id, _) => *account_id,
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

// TRANSACTION EXECUTION RESULT
// --------------------------------------------------------------------------------------------

/// Represents the result of executing a transaction by the client
///  
/// It contains an [ExecutedTransaction] and a list of [Note] that describe the details of the
/// notes created by the transaction execution
pub struct TransactionExecutionResult {
    executed_transaction: ExecutedTransaction,
    created_notes: Vec<Note>,
}

impl TransactionExecutionResult {
    pub fn new(executed_transaction: ExecutedTransaction, created_notes: Vec<Note>) -> Self {
        Self {
            executed_transaction,
            created_notes,
        }
    }

    pub fn executed_transaction(&self) -> &ExecutedTransaction {
        &self.executed_transaction
    }

    pub fn created_notes(&self) -> &Vec<Note> {
        &self.created_notes
    }
}

pub struct TransactionStub {
    pub id: Digest,
    pub account_id: AccountId,
    pub init_account_state: Digest,
    pub final_account_state: Digest,
    pub input_note_nullifiers: Vec<Digest>,
    pub output_notes: OutputNotes<NoteEnvelope>,
    pub transaction_script: Option<TransactionScript>,
    pub block_num: u32,
    pub committed: bool,
    pub commit_height: u64,
}

impl TransactionStub {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Digest,
        account_id: AccountId,
        init_account_state: Digest,
        final_account_state: Digest,
        input_note_nullifiers: Vec<Digest>,
        output_notes: OutputNotes<NoteEnvelope>,
        transaction_script: Option<TransactionScript>,
        block_num: u32,
        committed: bool,
        commit_height: u64,
    ) -> TransactionStub {
        TransactionStub {
            id,
            account_id,
            init_account_state,
            final_account_state,
            input_note_nullifiers,
            output_notes,
            transaction_script,
            block_num,
            committed,
            commit_height,
        }
    }
}

impl Client {
    // TRANSACTION DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Retrieves tracked transactions, filtered by [TransactionFilter].
    pub fn get_transactions(
        &self,
        transaction_filter: TransactionFilter,
    ) -> Result<Vec<TransactionStub>, ClientError> {
        self.store
            .get_transactions(transaction_filter)
            .map_err(|err| err.into())
    }

    // TRANSACTION
    // --------------------------------------------------------------------------------------------

    /// Creates and executes a transaction specified by the template, but does not change the
    /// local database.
    pub fn new_transaction(
        &mut self,
        transaction_template: TransactionTemplate,
    ) -> Result<TransactionExecutionResult, ClientError> {
        match transaction_template {
            TransactionTemplate::PayToId(PaymentTransactionData {
                asset: fungible_asset,
                sender_account_id,
                target_account_id,
            }) => self.new_p2id_transaction(fungible_asset, sender_account_id, target_account_id),
            TransactionTemplate::PayToIdWithRecall(_payment_data, _recall_height) => todo!(),
            TransactionTemplate::ConsumeNote(account_id, note_id) => {
                self.new_consume_notes_transaction(account_id, note_id)
            }
            TransactionTemplate::MintFungibleAsset {
                asset,
                target_account_id,
            } => self.new_mint_fungible_asset_transaction(asset, target_account_id),
        }
    }

    fn new_consume_notes_transaction(
        &mut self,
        account_id: AccountId,
        note_id: Option<NoteId>,
    ) -> Result<TransactionExecutionResult, ClientError> {
        self.tx_executor
            .load_account(account_id)
            .map_err(ClientError::TransactionExecutionError)?;

        let tx_script_code = ProgramAst::parse(
            "
            use.miden::contracts::auth::basic->auth_tx
    
            begin
                call.auth_tx::auth_tx_rpo_falcon512
            end
            ",
        )
        .unwrap();

        let account_auth = self.get_account_auth(account_id)?;

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

        let input_note = if note_id.is_some() {
            self.store.get_input_note_by_id(note_id.unwrap())?
        } else {
            let input_notes = self
                .store
                .get_input_notes(InputNoteFilter::Committed)
                .map_err(ClientError::StoreError)?;

            input_notes
                .first()
                .ok_or(ClientError::NoConsumableNoteForAccount(account_id))?
                .clone()
        };

        let tx_script = self
            .tx_executor
            .compile_tx_script(tx_script_code, script_inputs, vec![])
            .map_err(ClientError::TransactionExecutionError)?;

        // TODO: Change this to last block number after we confirm this execution works
        let block_num = input_note.inclusion_proof().unwrap().origin().block_num;

        // Execute the transaction and get the witness
        let executed_transaction = self
            .tx_executor
            .execute_transaction(
                account_id,
                block_num,
                &[input_note.note_id()],
                Some(tx_script.clone()),
            )
            .map_err(ClientError::TransactionExecutionError)?;

        Ok(TransactionExecutionResult::new(
            executed_transaction,
            vec![],
        ))
    }

    /// Creates and executes a mint transaction specified by the template.
    fn new_mint_fungible_asset_transaction(
        &mut self,
        asset: FungibleAsset,
        target_id: AccountId,
    ) -> Result<TransactionExecutionResult, ClientError> {
        let faucet_id = asset.faucet_id();

        // Construct Account
        let faucet_auth = self.get_account_auth(faucet_id)?;
        self.tx_executor
            .load_account(faucet_id)
            .map_err(ClientError::TransactionExecutionError)?;

        let block_ref = self.get_sync_height()?;

        let random_coin = self.get_random_coin();

        let created_note = create_p2id_note(faucet_id, target_id, vec![asset.into()], random_coin)
            .map_err(ClientError::NoteError)?;

        let recipient = created_note
            .recipient()
            .iter()
            .map(|x| x.as_int().to_string())
            .collect::<Vec<_>>()
            .join(".");

        let tx_script_code = ProgramAst::parse(
            format!(
                "
                use.miden::contracts::faucets::basic_fungible->faucet
                use.miden::contracts::auth::basic->auth_tx
    
                begin
                    push.{recipient}
                    push.{tag}
                    push.{amount}
                    call.faucet::distribute
    
                    call.auth_tx::auth_tx_rpo_falcon512
                    dropw dropw
    
                end
            ",
                recipient = recipient,
                tag = Felt::new(Into::<u64>::into(target_id)),
                amount = Felt::new(asset.amount()),
            )
            .as_str(),
        )
        .expect("program is well formed");

        let (pubkey_input, advice_map): (Word, Vec<Felt>) = match faucet_auth {
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
            .compile_tx_script(tx_script_code, script_inputs, vec![])
            .map_err(ClientError::TransactionExecutionError)?;

        // Execute the transaction and get the witness
        let executed_transaction = self
            .tx_executor
            .execute_transaction(faucet_id, block_ref, &[], Some(tx_script.clone()))
            .map_err(ClientError::TransactionExecutionError)?;

        Ok(TransactionExecutionResult::new(
            executed_transaction,
            vec![created_note],
        ))
    }

    fn new_p2id_transaction(
        &mut self,
        fungible_asset: Asset,
        sender_account_id: AccountId,
        target_account_id: AccountId,
    ) -> Result<TransactionExecutionResult, ClientError> {
        let random_coin = self.get_random_coin();

        let created_note = create_p2id_note(
            sender_account_id,
            target_account_id,
            vec![fungible_asset],
            random_coin,
        )
        .map_err(ClientError::NoteError)?;

        self.tx_executor
            .load_account(sender_account_id)
            .map_err(ClientError::TransactionExecutionError)?;

        let block_ref = self.get_sync_height()?;

        let recipient = created_note
            .recipient()
            .iter()
            .map(|x| x.as_int().to_string())
            .collect::<Vec<_>>()
            .join(".");

        let tx_script_code = ProgramAst::parse(&format!(
            "
        use.miden::contracts::auth::basic->auth_tx
        use.miden::contracts::wallets::basic->wallet

        begin
            push.{recipient}
            push.{tag}
            push.{asset}
            call.wallet::send_asset drop
            dropw dropw
            call.auth_tx::auth_tx_rpo_falcon512
        end
        ",
            recipient = recipient,
            tag = Felt::new(Into::<u64>::into(target_account_id)),
            asset = prepare_word(&fungible_asset.into()),
        ))
        .expect("program is correctly written");

        let account_auth = self
            .store
            .get_account_auth(sender_account_id)
            .map_err(ClientError::StoreError)?;
        let (pubkey_input, advice_map): (Word, Vec<Felt>) = match account_auth {
            AuthInfo::RpoFalcon512(key) => (
                key.public_key().into(),
                key.to_bytes()
                    .iter()
                    .map(|a| Felt::new(*a as u64))
                    .collect::<Vec<Felt>>(),
            ),
        };

        let tx_script_target = self
            .tx_executor
            .compile_tx_script(
                tx_script_code.clone(),
                vec![(pubkey_input, advice_map)],
                vec![],
            )
            .map_err(ClientError::TransactionExecutionError)?;

        // Execute the transaction and get the witness
        let executed_transaction = self
            .tx_executor
            .execute_transaction(
                sender_account_id,
                block_ref,
                &[],
                Some(tx_script_target.clone()),
            )
            .map_err(ClientError::TransactionExecutionError)?;

        Ok(TransactionExecutionResult::new(
            executed_transaction,
            vec![created_note],
        ))
    }

    /// Proves the specified transaction witness, submits it to the node, and stores the transaction in
    /// the local database for tracking.
    pub async fn send_transaction(
        &mut self,
        account_id: AccountId,
        transaction_execution_result: TransactionExecutionResult,
    ) -> Result<(), ClientError> {
        let transaction_prover = TransactionProver::new(ProvingOptions::default());
        let proven_transaction = transaction_prover
            .prove_transaction(transaction_execution_result.executed_transaction().clone())
            .map_err(ClientError::TransactionProvingError)?;

        println!("Proved transaction, submitting to the node...");

        self.submit_proven_transaction_request(proven_transaction.clone())
            .await?;

        // transaction was proven and submitted to the node correctly, persist note details and update account
        self.store.insert_proven_and_submitted_transaction_data(
            account_id,
            proven_transaction,
            transaction_execution_result.executed_transaction().clone(),
            transaction_execution_result.created_notes(),
        )?;

        Ok(())
    }

    async fn submit_proven_transaction_request(
        &mut self,
        proven_transaction: ProvenTransaction,
    ) -> Result<SubmitProvenTransactionResponse, ClientError> {
        let request = SubmitProvenTransactionRequest {
            transaction: proven_transaction.to_bytes(),
        };

        Ok(self
            .rpc_api
            .submit_proven_transaction(request)
            .await?
            .into_inner())
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Gets [RpoRandomCoin] from the client
    fn get_random_coin(&self) -> RpoRandomCoin {
        // TODO: Initialize coin status once along with the client and persist status for retrieval
        let mut rng = rand::thread_rng();
        let coin_seed: [u64; 4] = rng.gen();

        RpoRandomCoin::new(coin_seed.map(|x| x.into()))
    }
}

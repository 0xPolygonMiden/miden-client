use crypto::utils::Serializable;
use miden_lib::notes::{create_note, Script};
use miden_node_proto::{
    requests::SubmitProvenTransactionRequest, responses::SubmitProvenTransactionResponse,
};
use miden_tx::{ProvingOptions, TransactionProver};
use objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    assets::Asset,
    transaction::{ExecutedTransaction, OutputNotes, ProvenTransaction, TransactionScript},
    Digest,
};
use rand::Rng;

use crate::{
    errors::{ClientError, RpcApiError},
    store::mock_executor_data_store::{self, MockDataStore},
};

use super::Client;

pub enum TransactionTemplate {
    /// Creates a pay-to-id note directed to a specific account from a faucet
    PayToId(PaymentTransactionData),
    /// Creates a pay-to-id note directed to a specific account, specifying a block height at which the payment is recalled
    PayToIdWithRecall(PaymentTransactionData, u32),
    /// Consume all outstanding notes for an account
    ConsumeNotes(AccountId),
}

// PAYMENT TRANSACTION DATA
// --------------------------------------------------------------------------------------------

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
}

pub struct TransactionStub {
    pub id: Digest,
    pub account_id: AccountId,
    pub init_account_state: Digest,
    pub final_account_state: Digest,
    pub input_note_nullifiers: Vec<Digest>,
    pub output_notes: OutputNotes,
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
        output_notes: OutputNotes,
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

    /// Returns input notes managed by this client.
    pub fn get_transactions(&self) -> Result<Vec<TransactionStub>, ClientError> {
        self.store.get_transactions().map_err(|err| err.into())
    }

    // TRANSACTION
    // --------------------------------------------------------------------------------------------

    /// Creates and executes a transactions specified by the template, but does not change the
    /// local database.
    pub fn new_transaction(
        &mut self,
        transaction_template: TransactionTemplate,
    ) -> Result<ExecutedTransaction, ClientError> {
        match transaction_template {
            TransactionTemplate::PayToId(PaymentTransactionData {
                asset: fungible_asset,
                sender_account_id,
                target_account_id,
            }) => self.new_p2id_transaction(fungible_asset, sender_account_id, target_account_id),
            TransactionTemplate::PayToIdWithRecall(_payment_data, _recall_height) => todo!(),
            TransactionTemplate::ConsumeNotes(_) => todo!(),
        }
    }

    fn new_p2id_transaction(
        &mut self,
        fungible_asset: Asset,
        sender_account_id: AccountId,
        target_account_id: AccountId,
    ) -> Result<ExecutedTransaction, ClientError> {
        let p2id_script = Script::P2ID {
            target: target_account_id,
        };

        let mut rng = rand::thread_rng();
        let serial_numbers: [u64; 4] = rng.gen();

        let note = create_note(
            p2id_script,
            vec![fungible_asset],
            sender_account_id,
            Some(target_account_id.into()),
            serial_numbers.map(|number| number.into()),
        )
        .map_err(ClientError::NoteError)?;

        // TODO: Remove this as DataStore is implemented on the Client's Store

        #[cfg(feature = "testing")]
        {
            let (target_pub_key, _target_sk_pk_felt) =
                mock_executor_data_store::get_new_key_pair_with_advice_map();
            let target_account = mock_executor_data_store::get_account_with_default_account_code(
                target_account_id,
                target_pub_key,
                None,
            );
            let data_store: MockDataStore = MockDataStore::with_existing(
                Some(target_account.clone()),
                Some(vec![note.clone()]),
            );

            self.set_data_store(data_store.clone());
        }

        self.tx_executor
            .load_account(target_account_id)
            .map_err(ClientError::TransactionExecutionError)?;

        let block_ref = self.get_latest_block_num()?;
        let note_origins = [];

        let tx_script_code = ProgramAst::parse(
            "
            use.miden::auth::basic->auth_tx

            begin
                call.auth_tx::auth_tx_rpo_falcon512
            end
            ",
        )
        .expect("program is correctly written");

        let tx_script_target = self
            .tx_executor
            .compile_tx_script(
                tx_script_code.clone(),
                vec![/*(target_pub_key, target_sk_pk_felt)*/],
                vec![],
            )
            .map_err(ClientError::TransactionExecutionError)?;

        // Execute the transaction and get the witness
        let executed_transaction = self
            .tx_executor
            .execute_transaction(
                target_account_id,
                block_ref,
                &note_origins,
                Some(tx_script_target.clone()),
            )
            .map_err(ClientError::TransactionExecutionError)?;

        Ok(executed_transaction)
    }

    /// Proves the specified transaction witness, submits it to the node, and stores the transaction in
    /// the local database for tracking.
    pub async fn send_transaction(
        &mut self,
        transaction_execution_result: ExecutedTransaction,
    ) -> Result<(), ClientError> {
        let transaction_prover = TransactionProver::new(ProvingOptions::default());
        let proven_transaction = transaction_prover
            .prove_transaction(transaction_execution_result.clone())
            .map_err(ClientError::TransactionProvingError)?;

        self.submit_proven_transaction_request(proven_transaction.clone())
            .await?;

        self.store
            .insert_proven_transaction_data(proven_transaction, transaction_execution_result)?;

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
            .await
            .map_err(|err| ClientError::RpcApiError(RpcApiError::RequestError(err)))?
            .into_inner())
    }
}

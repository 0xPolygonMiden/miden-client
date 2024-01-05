use crypto::{utils::Serializable, Felt, Word};
use miden_lib::notes::{create_note, Script};
use miden_node_proto::{
    requests::SubmitProvenTransactionRequest, responses::SubmitProvenTransactionResponse,
};
use miden_tx::{ProvingOptions, TransactionProver};
use objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    assets::{Asset, FungibleAsset},
    transaction::{ExecutedTransaction, OutputNotes, ProvenTransaction, TransactionScript},
    Digest,
};
use rand::Rng;

use crate::{
    errors::{ClientError, RpcApiError},
    store::accounts::AuthInfo,
};

use super::Client;

pub enum TransactionTemplate {
    /// Consume all outstanding notes for an account
    ConsumeNotes(AccountId),
    // NOTE: Maybe this should be called "distribute"?
    /// Mint fungible assets using a faucet account
    MintFungibleAsset {
        asset: FungibleAsset,
        tag: u64,
        target_account_id: AccountId,
    },
    /// Creates a pay-to-id note directed to a specific account from a faucet
    PayToId(PaymentTransactionData),
    /// Creates a pay-to-id note directed to a specific account, specifying a block height at which the payment is recalled
    PayToIdWithRecall(PaymentTransactionData, u32),
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
            TransactionTemplate::MintFungibleAsset {
                asset,
                tag: _tag,
                target_account_id,
            } => self.new_mint_fungible_asset_transaction(asset, target_account_id),
        }
    }

    pub fn new_mint_fungible_asset_transaction(
        &mut self,
        asset: FungibleAsset,
        target_id: AccountId,
    ) -> Result<ExecutedTransaction, ClientError> {
        println!("{:?}", asset);
        println!("{:?}", target_id);
        let faucet_id = asset.faucet_id();
        let (faucet_account, _faucet_seed) = self.get_account_by_id(faucet_id)?;
        let faucet_auth = self.get_account_auth(faucet_id)?;

        self.tx_executor
            .load_account(faucet_account.id())
            .map_err(ClientError::TransactionExecutionError)?;

        let block_ref = self.get_latest_block_num()?;

        let mut rng = rand::thread_rng();
        let serial_num: [u64; 4] = rng.gen();

        let output_note = create_note(
            Script::P2ID { target: target_id },
            vec![asset.into()],
            faucet_id,
            Some(target_id.into()),
            serial_num.map(|n| n.into()),
        )
        .map_err(ClientError::NoteError)?;
        let amount = Felt::new(asset.amount());

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
                recipient = output_note.recipient(),
                tag = Felt::new(target_id.into()),
                amount = amount,
            )
            .as_str(),
        )
        .unwrap();

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
        let transaction_result = self
            .tx_executor
            .execute_transaction(faucet_account.id(), block_ref, &[], Some(tx_script.clone()))
            .map_err(ClientError::TransactionExecutionError)?;

        Ok(transaction_result)
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

        let _note = create_note(
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
                crate::store::mock_executor_data_store::get_new_key_pair_with_advice_map();
            let target_account =
                crate::store::mock_executor_data_store::get_account_with_default_account_code(
                    target_account_id,
                    target_pub_key,
                    None,
                );
            let data_store = crate::store::mock_executor_data_store::MockDataStore::with_existing(
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

// mod tests {

//     #[tokio::test]
//     pub async fn test_mint() {
//         let store_path = create_test_store_path();

//         let mut client = Client::new(ClientConfig::new(
//             "~/repos/miden-client/target/debug/store.sqlite3".to_string(),
//             Endpoint::default(),
//         ))
//         .await
//         .unwrap();

//         let asset: FungibleAsset =
//             FungibleAsset::new(AccountId::from_hex("0x871e602b3953f7cb").unwrap(), 100u64).unwrap();
//         let target_id = AccountId::from_hex("0x168187d729b32a84").unwrap();
//         client
//             .new_mint_fungible_asset_transaction(asset, target_id)
//             .unwrap();
//     }
// }

use miden_client::{
    ClientError,
    note::{get_input_note_with_id_prefix, BlockNumber},
    transaction::{
        DataStoreError,
        PaymentTransactionData, SwapTransactionData,
        TransactionExecutorError,
        TransactionRequestBuilder as NativeTransactionRequestBuilder,
        TransactionResult as NativeTransactionResult,
    },
};
use miden_lib::note::utils::build_swap_tag;
use miden_objects::{account::AccountId as NativeAccountId, asset::FungibleAsset, TransactionInputError};
use wasm_bindgen::prelude::*;

use crate::{
    models::{
        account_id::AccountId, note_type::NoteType, provers::TransactionProver,
        transaction_request::TransactionRequest, transaction_result::TransactionResult,
        transactions::NewSwapTransactionResult,
    },
    WebClient,
};

#[wasm_bindgen]
impl WebClient {
    pub async fn new_transaction(
        &mut self,
        account_id: &AccountId,
        transaction_request: &TransactionRequest,
    ) -> Result<TransactionResult, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let native_transaction_execution_result: NativeTransactionResult = client
                .new_transaction(account_id.into(), transaction_request.into())
                .await
                .map_err(|err| {
                    JsValue::from_str(&format!("Failed to execute New Transaction: {}", err))
                })?;

            Ok(native_transaction_execution_result.into())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn submit_transaction(
        &mut self,
        transaction_result: &TransactionResult,
    ) -> Result<(), JsValue> {
        let remote_prover = self.remote_prover.clone();
        if let Some(client) = self.get_mut_inner() {
            let native_transaction_result: NativeTransactionResult = transaction_result.into();
            match remote_prover {
                Some(ref remote_prover) => {
                    client
                        .submit_transaction_with_prover(
                            native_transaction_result,
                            remote_prover.clone(),
                        )
                        .await
                        .map_err(|err| {
                            JsValue::from_str(&format!("Failed to submit Transaction: {}", err))
                        })?;
                },
                None => {
                    client.submit_transaction(native_transaction_result).await.map_err(|err| {
                        JsValue::from_str(&format!("Failed to submit Transaction: {}", err))
                    })?;
                },
            }

            Ok(())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn submit_transaction_with_prover(
        &mut self,
        transaction_result: &TransactionResult,
        prover: TransactionProver,
    ) -> Result<(), JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let native_transaction_result: NativeTransactionResult = transaction_result.into();
            client
                .submit_transaction_with_prover(native_transaction_result, prover.get_prover())
                .await
                .map_err(|err| {
                    JsValue::from_str(&format!("Failed to submit Transaction: {}", err))
                })?;

            Ok(())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn new_mint_transaction(
        &mut self,
        target_account_id: &AccountId,
        faucet_id: &AccountId,
        note_type: &NoteType,
        amount: u64,
    ) -> Result<TransactionResult, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let fungible_asset = FungibleAsset::new(faucet_id.into(), amount).map_err(|err| {
                JsValue::from_str(&format!("Failed to create Fungible Asset: {}", err))
            })?;

            let mint_transaction_request = NativeTransactionRequestBuilder::mint_fungible_asset(
                fungible_asset,
                target_account_id.into(),
                note_type.into(),
                client.rng(),
            )
            .map_err(|err| {
                JsValue::from_str(&format!("Failed to create Mint Transaction Request: {}", err))
            })?
            .build();

            let mint_transaction_execution_result = client
                .new_transaction(faucet_id.into(), mint_transaction_request)
                .await
                .map_err(|err| {
                    let detailed_message = match &err {
                        ClientError::TransactionExecutorError(tx_err) => match tx_err {
                            TransactionExecutorError::TransactionProgramExecutionFailed(execution_err) => 
                    format!("Transaction execution failed: {}", execution_err),
                
                    TransactionExecutorError::FetchTransactionInputsFailed(data_store_err) => match data_store_err {
                        DataStoreError::InvalidTransactionInput(input_err) => match input_err {
                            TransactionInputError::InputNoteNotInBlock(note_id, block_number) => 
                                format!("Transaction input error: Input note {} was not created in block {}", note_id, block_number),
    
                            TransactionInputError::DuplicateInputNote(nullifier) => 
                                format!("Transaction input error: Duplicate input note with nullifier {}", nullifier),
    
                            TransactionInputError::TooManyInputNotes(count) => 
                                format!("Transaction input error: Too many input notes ({}) exceeds the maximum allowed", count),
    
                            _ => format!("Transaction input error: {}", input_err),
                        },
    
                        _ => format!("Fetching transaction inputs failed: {}", data_store_err), // Default case for other DataStoreErrors
                    },
                
                TransactionExecutorError::InconsistentAccountId { input_id, output_id } => 
                    format!("Account ID mismatch: input = {}, output = {}", input_id, output_id),
                
                TransactionExecutorError::InconsistentAccountNonceDelta { expected, actual } => 
                    format!("Nonce mismatch: expected {:?}, found {:?}", expected, actual),
                
                TransactionExecutorError::TransactionOutputConstructionFailed(output_err) => 
                    format!("Transaction output construction failed: {}", output_err),
                
                TransactionExecutorError::TransactionHostCreationFailed(host_err) => 
                    format!("Transaction host creation failed: {}", host_err),
                        }
                        _ => format!("Failed to execute Mint Transaction: {}", err),
                    };
            
                    JsValue::from_str(&detailed_message)
                })?;

            let result = mint_transaction_execution_result.clone().into();

            client
                .submit_transaction(mint_transaction_execution_result)
                .await
                .map_err(|err| {
                    JsValue::from_str(&format!("Failed to submit Mint Transaction: {}", err))
                })?;

            Ok(result)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn new_send_transaction(
        &mut self,
        sender_account_id: &AccountId,
        target_account_id: &AccountId,
        faucet_id: &AccountId,
        note_type: &NoteType,
        amount: u64,
        recall_height: Option<u32>,
    ) -> Result<TransactionResult, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let fungible_asset = FungibleAsset::new(faucet_id.into(), amount).map_err(|err| {
                JsValue::from_str(&format!("Failed to create Fungible Asset: {}", err))
            })?;

            let payment_transaction = PaymentTransactionData::new(
                vec![fungible_asset.into()],
                sender_account_id.into(),
                target_account_id.into(),
            );

            let send_transaction_request = if let Some(recall_height) = recall_height {
                NativeTransactionRequestBuilder::pay_to_id(
                    payment_transaction,
                    Some(BlockNumber::from(recall_height)),
                    note_type.into(),
                    client.rng(),
                )
                .map_err(|err| {
                    JsValue::from_str(&format!(
                        "Failed to create Send Transaction Request with Recall Height: {}",
                        err
                    ))
                })?
                .build()
            } else {
                NativeTransactionRequestBuilder::pay_to_id(
                    payment_transaction,
                    None,
                    note_type.into(),
                    client.rng(),
                )
                .map_err(|err| {
                    JsValue::from_str(&format!(
                        "Failed to create Send Transaction Request: {}",
                        err
                    ))
                })?
                .build()
            };

            let send_transaction_execution_result = client
                .new_transaction(sender_account_id.into(), send_transaction_request)
                .await
                .map_err(|err| {
                    JsValue::from_str(&format!("Failed to execute Send Transaction: {}", err))
                })?;

            let result = send_transaction_execution_result.clone().into();

            client
                .submit_transaction(send_transaction_execution_result)
                .await
                .map_err(|err| {
                    JsValue::from_str(&format!("Failed to submit Mint Transaction: {}", err))
                })?;

            Ok(result)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn new_consume_transaction(
        &mut self,
        account_id: &AccountId,
        list_of_note_ids: Vec<String>,
    ) -> Result<TransactionResult, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            web_sys::console::log_1(&JsValue::from_str("Consume Transaction"));
            let mut result = Vec::new();
            for note_id in list_of_note_ids {
                let note_record =
                    get_input_note_with_id_prefix(client, &note_id).await.map_err(|err| {
                        JsValue::from_str(&format!("Failed to get input note: {}", err))
                    })?;
                result.push(note_record.id());
            }
            web_sys::console::log_1(&JsValue::from_str("Consume Transaction 2"));

            let consume_transaction_request =
                NativeTransactionRequestBuilder::consume_notes(result).build();
            web_sys::console::log_1(&JsValue::from_str("Consume Transaction 3"));

            let consume_transaction_execution_result = client
                .new_transaction(account_id.into(), consume_transaction_request)
                .await
                .map_err(|err| {
                    JsValue::from_str(&format!("Failed to execute Consume Transaction: {}", err))
                })?;
            web_sys::console::log_1(&JsValue::from_str("Consume Transaction 4"));

            let result = consume_transaction_execution_result.clone().into();
            web_sys::console::log_1(&JsValue::from_str("Consume Transaction 5"));

            client.submit_transaction(consume_transaction_execution_result).await.map_err(
                |err| JsValue::from_str(&format!("Failed to submit Consume Transaction: {}", err)),
            )?;
            web_sys::console::log_1(&JsValue::from_str("Consume Transaction 6"));

            Ok(result)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn new_swap_transaction(
        &mut self,
        sender_account_id: String,
        offered_asset_faucet_id: String,
        offered_asset_amount: String,
        requested_asset_faucet_id: String,
        requested_asset_amount: String,
        note_type: &NoteType,
    ) -> Result<NewSwapTransactionResult, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let sender_account_id = NativeAccountId::from_hex(&sender_account_id).unwrap();

            let offered_asset_faucet_id =
                NativeAccountId::from_hex(&offered_asset_faucet_id).unwrap();
            let offered_asset_amount_as_u64: u64 =
                offered_asset_amount.parse::<u64>().map_err(|err| err.to_string())?;
            let offered_fungible_asset =
                FungibleAsset::new(offered_asset_faucet_id, offered_asset_amount_as_u64)
                    .map_err(|err| err.to_string())?
                    .into();

            let requested_asset_faucet_id =
                NativeAccountId::from_hex(&requested_asset_faucet_id).unwrap();
            let requested_asset_amount_as_u64: u64 =
                requested_asset_amount.parse::<u64>().map_err(|err| err.to_string())?;
            let requested_fungible_asset =
                FungibleAsset::new(requested_asset_faucet_id, requested_asset_amount_as_u64)
                    .map_err(|err| err.to_string())?
                    .into();

            let swap_transaction = SwapTransactionData::new(
                sender_account_id,
                offered_fungible_asset,
                requested_fungible_asset,
            );

            let swap_transaction_request = NativeTransactionRequestBuilder::swap(
                swap_transaction.clone(),
                note_type.into(),
                client.rng(),
            )
            .unwrap()
            .build();
            let swap_transaction_execution_result = client
                .new_transaction(sender_account_id, swap_transaction_request.clone())
                .await
                .unwrap();
            let mut result = NewSwapTransactionResult::new(
                swap_transaction_execution_result.executed_transaction().id().to_string(),
                swap_transaction_request
                    .expected_output_notes()
                    .map(|note| note.id().to_string())
                    .collect(),
                swap_transaction_request
                    .expected_future_notes()
                    .map(|note| note.0.id().to_string())
                    .collect(),
                None,
            );

            client.submit_transaction(swap_transaction_execution_result).await.unwrap();

            let payback_note_tag_u32: u32 = build_swap_tag(
                note_type.into(),
                &swap_transaction.offered_asset(),
                &swap_transaction.requested_asset(),
            )
            .map_err(|err| err.to_string())?
            .into();

            result.set_note_tag(payback_note_tag_u32.to_string());

            Ok(result)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

use miden_client::{
    note::{BlockNumber, get_input_note_with_id_prefix},
    transaction::{
        PaymentTransactionData, SwapTransactionData,
        TransactionRequestBuilder as NativeTransactionRequestBuilder,
        TransactionResult as NativeTransactionResult,
    },
};
use miden_lib::note::utils::build_swap_tag;
use miden_objects::{account::AccountId as NativeAccountId, asset::FungibleAsset};
use wasm_bindgen::prelude::*;

use crate::{
    WebClient, js_error_with_context,
    models::{
        account_id::AccountId, note_type::NoteType, provers::TransactionProver,
        transaction_request::TransactionRequest, transaction_result::TransactionResult,
        transactions::NewSwapTransactionResult,
    },
};

#[wasm_bindgen]
impl WebClient {
    #[wasm_bindgen(js_name = "newTransaction")]
    pub async fn new_transaction(
        &mut self,
        account_id: &AccountId,
        transaction_request: &TransactionRequest,
    ) -> Result<TransactionResult, JsValue> {
        self.fetch_and_cache_account_auth_by_account_id(account_id).await?;

        if let Some(client) = self.get_mut_inner() {
            let native_transaction_execution_result: NativeTransactionResult = client
                .new_transaction(account_id.into(), transaction_request.into())
                .await
                .map_err(|err| js_error_with_context(err, "failed to create new transaction"))?;

            Ok(native_transaction_execution_result.into())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    #[wasm_bindgen(js_name = "submitTransaction")]
    pub async fn submit_transaction(
        &mut self,
        transaction_result: &TransactionResult,
        prover: Option<TransactionProver>,
    ) -> Result<(), JsValue> {
        let native_transaction_result: NativeTransactionResult = transaction_result.into();

        if let Some(client) = self.get_mut_inner() {
            match prover {
                Some(p) => {
                    client
                        .submit_transaction_with_prover(native_transaction_result, p.get_prover())
                        .await
                        .map_err(|err| {
                            js_error_with_context(err, "failed to submit transaction with prover")
                        })?;
                },
                None => {
                    client.submit_transaction(native_transaction_result).await.map_err(|err| {
                        js_error_with_context(err, "failed to submit transaction")
                    })?;
                },
            }
            Ok(())
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    #[wasm_bindgen(js_name = "newMintTransaction")]
    pub async fn new_mint_transaction(
        &mut self,
        target_account_id: &AccountId,
        faucet_id: &AccountId,
        note_type: &NoteType,
        amount: u64,
    ) -> Result<TransactionResult, JsValue> {
        let fungible_asset = FungibleAsset::new(faucet_id.into(), amount)
            .map_err(|err| js_error_with_context(err, "failed to create fungible asset"))?;

        let mint_transaction_request = {
            let client = self.get_mut_inner().ok_or_else(|| {
                JsValue::from_str("Client not initialized while generating transaction request")
            })?;

            NativeTransactionRequestBuilder::mint_fungible_asset(
                fungible_asset,
                target_account_id.into(),
                note_type.into(),
                client.rng(),
            )
            .and_then(NativeTransactionRequestBuilder::build)
            .map_err(|err| {
                js_error_with_context(err, "failed to create mint transaction request")
            })?
        };

        self.execute_and_submit_transaction(faucet_id, &mint_transaction_request.into(), "Mint")
            .await
    }

    #[wasm_bindgen(js_name = "newSendTransaction")]
    pub async fn new_send_transaction(
        &mut self,
        sender_account_id: &AccountId,
        target_account_id: &AccountId,
        faucet_id: &AccountId,
        note_type: &NoteType,
        amount: u64,
        recall_height: Option<u32>,
    ) -> Result<TransactionResult, JsValue> {
        let fungible_asset = FungibleAsset::new(faucet_id.into(), amount)
            .map_err(|err| js_error_with_context(err, "failed to create fungible asset"))?;

        let payment_transaction = PaymentTransactionData::new(
            vec![fungible_asset.into()],
            sender_account_id.into(),
            target_account_id.into(),
        );

        let send_transaction_request = {
            let client = self.get_mut_inner().ok_or_else(|| {
                JsValue::from_str("Client not initialized while generating transaction request")
            })?;

            if let Some(recall_height) = recall_height {
                NativeTransactionRequestBuilder::pay_to_id(
                    payment_transaction,
                    Some(BlockNumber::from(recall_height)),
                    note_type.into(),
                    client.rng(),
                )
                .and_then(NativeTransactionRequestBuilder::build)
                .map_err(|err| {
                    js_error_with_context(
                        err,
                        "failed to create send transaction request with recall height",
                    )
                })?
            } else {
                NativeTransactionRequestBuilder::pay_to_id(
                    payment_transaction,
                    None,
                    note_type.into(),
                    client.rng(),
                )
                .and_then(NativeTransactionRequestBuilder::build)
                .map_err(|err| {
                    js_error_with_context(err, "failed to create send transaction request")
                })?
            }
        };

        self.execute_and_submit_transaction(
            sender_account_id,
            &send_transaction_request.into(),
            "Send",
        )
        .await
    }

    #[wasm_bindgen(js_name = "newConsumeTransaction")]
    pub async fn new_consume_transaction(
        &mut self,
        account_id: &AccountId,
        list_of_note_ids: Vec<String>,
    ) -> Result<TransactionResult, JsValue> {
        let consume_transaction_request = {
            let client = self.get_mut_inner().ok_or_else(|| {
                JsValue::from_str("Client not initialized while generating transaction request")
            })?;

            let mut result = Vec::new();
            for note_id in list_of_note_ids {
                let note_record = get_input_note_with_id_prefix(client, &note_id)
                    .await
                    .map_err(|err| js_error_with_context(err, "failed to get input note"))?;
                result.push(note_record.id());
            }

            NativeTransactionRequestBuilder::consume_notes(result).build().map_err(|err| {
                js_error_with_context(err, "failed to create consume transaction request")
            })?
        };

        self.execute_and_submit_transaction(
            account_id,
            &consume_transaction_request.into(),
            "Consume",
        )
        .await
    }

    #[wasm_bindgen(js_name = "newSwapTransaction")]
    pub async fn new_swap_transaction(
        &mut self,
        sender_account_id: String,
        offered_asset_faucet_id: String,
        offered_asset_amount: String,
        requested_asset_faucet_id: String,
        requested_asset_amount: String,
        note_type: &NoteType,
    ) -> Result<NewSwapTransactionResult, JsValue> {
        let sender_account_id =
            NativeAccountId::from_hex(&sender_account_id).map_err(|err| err.to_string())?;
        let offered_asset_faucet_id =
            NativeAccountId::from_hex(&offered_asset_faucet_id).map_err(|err| err.to_string())?;
        let offered_asset_amount_as_u64: u64 =
            offered_asset_amount.parse::<u64>().map_err(|err| err.to_string())?;
        let offered_fungible_asset =
            FungibleAsset::new(offered_asset_faucet_id, offered_asset_amount_as_u64)
                .map_err(|err| err.to_string())?
                .into();

        let requested_asset_faucet_id =
            NativeAccountId::from_hex(&requested_asset_faucet_id).map_err(|err| err.to_string())?;
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

        // TODO: Leaving this alone for now because new_swap_transaction needs a rework anyway
        self.fetch_and_cache_account_auth_by_account_id(&sender_account_id.into())
            .await?;

        if let Some(client) = self.get_mut_inner() {
            let swap_transaction_request = NativeTransactionRequestBuilder::swap(
                &swap_transaction,
                note_type.into(),
                client.rng(),
            )
            .map_err(|err| err.to_string())?
            .build()
            .map_err(|err| {
                js_error_with_context(err, "failed to create swap transaction request")
            })?;

            let swap_transaction_execution_result = client
                .new_transaction(sender_account_id, swap_transaction_request.clone())
                .await
                .map_err(|err| {
                    JsValue::from_str(&format!("failed to execute swap transaction: {err:?}"))
                })?;
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

            client
                .submit_transaction(swap_transaction_execution_result)
                .await
                .map_err(|err| js_error_with_context(err, "failed to submit swap transaction"))?;

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

    /// Helper function to execute a transaction and submit it.
    async fn execute_and_submit_transaction(
        &mut self,
        account_id: &AccountId,
        transaction_request: &TransactionRequest,
        transaction_type: &str, // For logging error messages
    ) -> Result<TransactionResult, JsValue> {
        let transaction_execution_result =
            self.new_transaction(account_id, transaction_request).await.map_err(|err| {
                JsValue::from_str(&format!(
                    "failed to create {transaction_type} transaction: {}",
                    err.as_string().expect("error message should be a string")
                ))
            })?;

        self.submit_transaction(&transaction_execution_result, None)
            .await
            .map_err(|err| {
                JsValue::from_str(&format!(
                    "failed to submit {transaction_type} transaction: {}",
                    err.as_string().expect("error message should be a string")
                ))
            })?;

        Ok(transaction_execution_result)
    }
}

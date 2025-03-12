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
    WebClient,
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
        self.fetch_and_cache_account_auth_by_account_id(account_id)
            .await
            .map_err(|err| {
                JsValue::from_str(&format!(
                    "Failed to fetch and cache account auth by account id for mint transaction: {err:?}"
                ))
            })?;

        if let Some(client) = self.get_mut_inner() {
            let native_transaction_execution_result: NativeTransactionResult = client
                .new_transaction(account_id.into(), transaction_request.into())
                .await
                .map_err(|err| {
                    JsValue::from_str(&format!("Failed to execute New Transaction: {err}"))
                })?;

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
                            JsValue::from_str(&format!(
                                "Failed to submit Transaction with prover: {err}"
                            ))
                        })?;
                },
                None => {
                    client.submit_transaction(native_transaction_result).await.map_err(|err| {
                        JsValue::from_str(&format!("Failed to submit Transaction: {err}"))
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
            .map_err(|err| JsValue::from_str(&format!("Failed to create Fungible Asset: {err}")))?;

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
                JsValue::from_str(&format!("Failed to create Mint Transaction Request: {err}"))
            })?
        };

        Ok(self.execute_and_submit_transaction(faucet_id, &mint_transaction_request.into(), "Mint").await?)
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
            .map_err(|err| JsValue::from_str(&format!("Failed to create Fungible Asset: {err}")))?;

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
                    JsValue::from_str(&format!(
                        "Failed to create Send Transaction Request with Recall Height: {err}"
                    ))
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
                    JsValue::from_str(&format!("Failed to create Send Transaction Request: {err}"))
                })?
            }
        };

        Ok(self.execute_and_submit_transaction(sender_account_id, &send_transaction_request.into(), "Send").await?)
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
                let note_record =
                    get_input_note_with_id_prefix(client, &note_id).await.map_err(|err| {
                        JsValue::from_str(&format!("Failed to get input note: {err}"))
                    })?;
                result.push(note_record.id());
            }

            NativeTransactionRequestBuilder::consume_notes(result).build().map_err(|err| {
                JsValue::from_str(&format!(
                    "Failed to create Consume Transaction Request: {err}"
                ))
            })?
        };

        Ok(self.execute_and_submit_transaction(account_id, &consume_transaction_request.into(), "Consume").await?)
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
        let sender_account_id = NativeAccountId::from_hex(&sender_account_id).unwrap();

        let offered_asset_faucet_id = NativeAccountId::from_hex(&offered_asset_faucet_id).unwrap();
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

        // TODO: Leaving this alone for now because new_swap_transaction needs a rework anyway
        self.fetch_and_cache_account_auth_by_account_id(&sender_account_id.into())
            .await
            .map_err(|err| {
                JsValue::from_str(&format!(
                    "Failed to fetch and cache account auth by account id for mint transaction: {err:?}"
                ))
            })?;

        if let Some(client) = self.get_mut_inner() {
            let swap_transaction_request = NativeTransactionRequestBuilder::swap(
                &swap_transaction,
                note_type.into(),
                client.rng(),
            )
            .unwrap()
            .build()
            .unwrap();
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

    /// Helper function to execute a transaction and submit it.
    async fn execute_and_submit_transaction(
        &mut self,
        account_id: &AccountId,
        transaction_request: &TransactionRequest,
        transaction_type: &str, // For logging error messages
    ) -> Result<TransactionResult, JsValue> {
        let transaction_execution_result = self
            .new_transaction(account_id, transaction_request)
            .await
            .map_err(|err| {
                JsValue::from_str(&format!(
                    "Failed to execute {transaction_type} Transaction: {err:?}"
                ))
            })?;

        self.submit_transaction(&transaction_execution_result, None)
            .await
            .map_err(|err| {
                JsValue::from_str(&format!(
                    "Failed to submit {transaction_type} Transaction: {err:?}"
                ))
            })?;

        Ok(transaction_execution_result)
    }
}

use miden_client::client::{
    build_swap_tag, get_input_note_with_id_prefix,
    transactions::transaction_request::{
        PaymentTransactionData, SwapTransactionData, TransactionTemplate,
    },
};
use miden_objects::{accounts::AccountId, assets::FungibleAsset, notes::NoteType as MidenNoteType};
use wasm_bindgen::prelude::*;

use super::WebClient;
use crate::web_client::models::transactions::{NewSwapTransactionResult, NewTransactionResult};

#[wasm_bindgen]
impl WebClient {
    pub async fn new_mint_transaction(
        &mut self,
        target_account_id: String,
        faucet_id: String,
        note_type: String,
        amount: String,
    ) -> Result<NewTransactionResult, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let target_account_id = AccountId::from_hex(&target_account_id).unwrap();
            let faucet_id = AccountId::from_hex(&faucet_id).unwrap();
            let amount_as_u64: u64 = amount.parse::<u64>().map_err(|err| err.to_string())?;
            let fungible_asset =
                FungibleAsset::new(faucet_id, amount_as_u64).map_err(|err| err.to_string())?;
            let note_type = match note_type.as_str() {
                "Public" => MidenNoteType::Public,
                "Private" => MidenNoteType::OffChain,
                _ => MidenNoteType::OffChain,
            };

            let mint_transaction_template = TransactionTemplate::MintFungibleAsset(
                fungible_asset,
                target_account_id,
                note_type,
            );

            let mint_transaction_request = client
                .build_transaction_request(mint_transaction_template.clone())
                .await
                .unwrap();

            let mint_transaction_execution_result =
                client.new_transaction(mint_transaction_request).await.unwrap();

            let result = NewTransactionResult::new(
                mint_transaction_execution_result.executed_transaction().id().to_string(),
                mint_transaction_execution_result
                    .created_notes()
                    .iter()
                    .map(|note| note.id().to_string())
                    .collect(),
            );
            let proven_transaction = client
                .prove_transaction(mint_transaction_execution_result.executed_transaction().clone())
                .unwrap();

            client
                .submit_transaction(mint_transaction_execution_result, proven_transaction)
                .await
                .unwrap();

            Ok(result)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn new_send_transaction(
        &mut self,
        sender_account_id: String,
        target_account_id: String,
        faucet_id: String,
        note_type: String,
        amount: String,
        recall_height: Option<String>,
    ) -> Result<NewTransactionResult, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let sender_account_id = AccountId::from_hex(&sender_account_id).unwrap();
            let target_account_id = AccountId::from_hex(&target_account_id).unwrap();
            let faucet_id = AccountId::from_hex(&faucet_id).unwrap();
            let amount_as_u64: u64 = amount.parse::<u64>().map_err(|err| err.to_string())?;
            let fungible_asset = FungibleAsset::new(faucet_id, amount_as_u64)
                .map_err(|err| err.to_string())?
                .into();

            let note_type = match note_type.as_str() {
                "Public" => MidenNoteType::Public,
                "Private" => MidenNoteType::OffChain,
                _ => MidenNoteType::OffChain,
            };
            let payment_transaction =
                PaymentTransactionData::new(fungible_asset, sender_account_id, target_account_id);

            let send_transaction_template: TransactionTemplate;
            if let Some(recall_height) = recall_height {
                let recall_height_as_u32: u32 =
                    recall_height.parse::<u32>().map_err(|err| err.to_string())?;
                send_transaction_template = TransactionTemplate::PayToIdWithRecall(
                    payment_transaction,
                    recall_height_as_u32,
                    note_type,
                );
            } else {
                send_transaction_template =
                    TransactionTemplate::PayToId(payment_transaction, note_type);
            }

            let send_transaction_request = client
                .build_transaction_request(send_transaction_template.clone())
                .await
                .unwrap();
            let send_transaction_execution_result =
                client.new_transaction(send_transaction_request).await.unwrap();
            let result = NewTransactionResult::new(
                send_transaction_execution_result.executed_transaction().id().to_string(),
                send_transaction_execution_result
                    .created_notes()
                    .iter()
                    .map(|note| note.id().to_string())
                    .collect(),
            );

            let proven_transaction = client
                .prove_transaction(send_transaction_execution_result.executed_transaction().clone())
                .unwrap();
            client
                .submit_transaction(send_transaction_execution_result, proven_transaction)
                .await
                .unwrap();

            Ok(result)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn new_consume_transaction(
        &mut self,
        account_id: String,
        list_of_notes: Vec<String>,
    ) -> Result<NewTransactionResult, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let account_id = AccountId::from_hex(&account_id).unwrap();
            let mut result = Vec::new();
            for note_id in list_of_notes {
                match get_input_note_with_id_prefix(client, &note_id).await {
                    Ok(note_record) => result.push(note_record.id()),
                    Err(err) => return Err(JsValue::from_str(&err.to_string())),
                }
            }

            let consume_transaction_template =
                TransactionTemplate::ConsumeNotes(account_id, result);

            let consume_transaction_request = client
                .build_transaction_request(consume_transaction_template.clone())
                .await
                .unwrap();
            let consume_transaction_execution_result =
                client.new_transaction(consume_transaction_request).await.unwrap();
            let result = NewTransactionResult::new(
                consume_transaction_execution_result.executed_transaction().id().to_string(),
                consume_transaction_execution_result
                    .created_notes()
                    .iter()
                    .map(|note| note.id().to_string())
                    .collect(),
            );
            let proven_transaction = client
                .prove_transaction(
                    consume_transaction_execution_result.executed_transaction().clone(),
                )
                .unwrap();
            client
                .submit_transaction(consume_transaction_execution_result, proven_transaction)
                .await
                .unwrap();

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
        note_type: String,
    ) -> Result<NewSwapTransactionResult, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let sender_account_id = AccountId::from_hex(&sender_account_id).unwrap();

            let offered_asset_faucet_id = AccountId::from_hex(&offered_asset_faucet_id).unwrap();
            let offered_asset_amount_as_u64: u64 =
                offered_asset_amount.parse::<u64>().map_err(|err| err.to_string())?;
            let offered_fungible_asset =
                FungibleAsset::new(offered_asset_faucet_id, offered_asset_amount_as_u64)
                    .map_err(|err| err.to_string())?
                    .into();

            let requested_asset_faucet_id =
                AccountId::from_hex(&requested_asset_faucet_id).unwrap();
            let requested_asset_amount_as_u64: u64 =
                requested_asset_amount.parse::<u64>().map_err(|err| err.to_string())?;
            let requested_fungible_asset =
                FungibleAsset::new(requested_asset_faucet_id, requested_asset_amount_as_u64)
                    .map_err(|err| err.to_string())?
                    .into();

            let note_type = match note_type.as_str() {
                "Public" => MidenNoteType::Public,
                "Private" => MidenNoteType::OffChain,
                _ => MidenNoteType::OffChain,
            };

            let swap_transaction = SwapTransactionData::new(
                sender_account_id,
                offered_fungible_asset,
                requested_fungible_asset,
            );

            let swap_transaction_template = TransactionTemplate::Swap(swap_transaction, note_type);

            let swap_transaction_request = client
                .build_transaction_request(swap_transaction_template.clone())
                .await
                .unwrap();
            let swap_transaction_execution_result =
                client.new_transaction(swap_transaction_request.clone()).await.unwrap();
            let mut result = NewSwapTransactionResult::new(
                swap_transaction_execution_result.executed_transaction().id().to_string(),
                swap_transaction_request
                    .expected_output_notes()
                    .iter()
                    .map(|note| note.id().to_string())
                    .collect(),
                swap_transaction_request
                    .expected_partial_notes()
                    .iter()
                    .map(|note| note.id().to_string())
                    .collect(),
                None,
            );
            let proven_transaction = client
                .prove_transaction(swap_transaction_execution_result.executed_transaction().clone())
                .unwrap();
            client
                .submit_transaction(swap_transaction_execution_result, proven_transaction)
                .await
                .unwrap();

            if let TransactionTemplate::Swap(swap_data, note_type) = swap_transaction_template {
                let payback_note_tag_u32: u32 = build_swap_tag(
                    note_type,
                    swap_data.offered_asset().faucet_id(),
                    swap_data.requested_asset().faucet_id(),
                )
                .map_err(|err| err.to_string())?
                .into();

                result.set_note_tag(payback_note_tag_u32.to_string());
            }

            Ok(result)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

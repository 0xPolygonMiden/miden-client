use miden_objects::{
    accounts::AccountId, 
    assets::FungibleAsset, 
    crypto::rand::FeltRng, 
    notes::{
        NoteId, NoteType as MidenNoteType
    }
};

use super::WebClient;
use crate::web_client::models::transactions::NewTransactionResult;

use crate::native_code::{
    errors::NoteIdPrefixFetchError, 
    rpc::NodeRpcClient, 
    store::{
        note_record::InputNoteRecord, 
        NoteFilter, 
        Store, 
        TransactionFilter
    }, 
    transactions::transaction_request::{
        PaymentTransactionData, TransactionTemplate
    }, Client
};

use serde::{Serialize, Deserialize};
use serde_wasm_bindgen::from_value;

use wasm_bindgen::prelude::*;
use web_sys::console;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TransactionType {
    /// Create a pay-to-id transaction.
    P2ID {
        sender_account_id: String,
        target_account_id: String,
        faucet_id: String,
        amount: String,
        note_type: String,
    },
    /// Mint `amount` tokens from the specified fungible faucet (corresponding to `faucet_id`). The created note can then be then consumed by
    /// `target_account_id`.
    Mint {
        target_account_id: String,
        faucet_id: String,
        amount: String,
        note_type: String,
    },
    /// Create a pay-to-id with recall transaction.
    P2IDR {
        sender_account_id: String,
        target_account_id: String,
        faucet_id: String,
        amount: String,
        recall_height: String,
        note_type: String,
    },
    /// Consume with the account corresponding to `account_id` all of the notes from `list_of_notes`.
    ConsumeNotes {
        account_id: String,
        /// A list of note IDs or the hex prefixes of their corresponding IDs
        list_of_notes: Vec<String>,
    },
}

#[wasm_bindgen]
impl WebClient {
    pub async fn get_transactions(
        &mut self
    ) -> Result<JsValue, JsValue> {
        if let Some(ref mut client) = self.get_mut_inner() {

            let transactions = client.get_transactions(TransactionFilter::All).await.unwrap();

            let transactionIds: Vec<String> = transactions.iter().map(|transaction| {
                transaction.id.to_string()
            }).collect();


            serde_wasm_bindgen::to_value(&transactionIds).map_err(|e| JsValue::from_str(&e.to_string()))
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn new_transaction(
        &mut self,
        transaction_type: JsValue
    ) -> Result<NewTransactionResult, JsValue> {
        if let Some(ref mut client) = self.get_mut_inner() {
            let transaction_type: TransactionType = from_value(transaction_type).unwrap();
            let transaction_template: TransactionTemplate = build_transaction_template(client, &transaction_type).await.unwrap();
            let transaction_request = client.build_transaction_request(transaction_template).await.unwrap();

            let transaction_execution_result = client.new_transaction(transaction_request).await.unwrap();
            let result = NewTransactionResult::new(
                transaction_execution_result.executed_transaction().id().to_string(),
                transaction_execution_result.created_notes().iter().map(|note| note.id().to_string()).collect()
            );

            client.submit_transaction(transaction_execution_result).await.unwrap();

            Ok(result)
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

fn parse_note_type(
    note_type: String
) -> MidenNoteType {
    match note_type.as_str() {
        "Public" => MidenNoteType::Public,
        "Private" => MidenNoteType::OffChain,
        _ => MidenNoteType::OffChain
    }
}

async fn build_transaction_template<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &Client<N, R, S>,
    transaction_type: &TransactionType,
) -> Result<TransactionTemplate, String> {
    match transaction_type {
        TransactionType::P2ID {
            sender_account_id,
            target_account_id,
            faucet_id,
            amount,
            note_type,
        } => {
            let note_type: MidenNoteType = parse_note_type(note_type.to_string());
            let amount_as_u64: u64 = amount.parse::<u64>().map_err(|err| err.to_string())?;

            let faucet_id = AccountId::from_hex(faucet_id).map_err(|err| err.to_string())?;
            let fungible_asset =
                FungibleAsset::new(faucet_id, amount_as_u64).map_err(|err| err.to_string())?.into();
            let sender_account_id =
                AccountId::from_hex(sender_account_id).map_err(|err| err.to_string())?;
            let target_account_id =
                AccountId::from_hex(target_account_id).map_err(|err| err.to_string())?;

            let payment_transaction =
                PaymentTransactionData::new(fungible_asset, sender_account_id, target_account_id);

            Ok(TransactionTemplate::PayToId(payment_transaction, note_type))
        },
        TransactionType::P2IDR {
            sender_account_id,
            target_account_id,
            faucet_id,
            amount,
            recall_height,
            note_type,
        } => {
            let note_type: MidenNoteType = parse_note_type(note_type.to_string());
            let amount_as_u64: u64 = amount.parse::<u64>().map_err(|err| err.to_string())?;
            let recall_height_as_u32: u32 = recall_height.parse::<u32>().map_err(|err| err.to_string())?;

            let faucet_id = AccountId::from_hex(faucet_id).map_err(|err| err.to_string())?;
            let fungible_asset =
                FungibleAsset::new(faucet_id, amount_as_u64).map_err(|err| err.to_string())?.into();
            let sender_account_id =
                AccountId::from_hex(sender_account_id).map_err(|err| err.to_string())?;
            let target_account_id =
                AccountId::from_hex(target_account_id).map_err(|err| err.to_string())?;

            let payment_transaction =
                PaymentTransactionData::new(fungible_asset, sender_account_id, target_account_id);
            Ok(TransactionTemplate::PayToIdWithRecall(
                payment_transaction,
                recall_height_as_u32,
                note_type,
            ))
        },
        TransactionType::Mint {
            faucet_id,
            target_account_id,
            amount,
            note_type,
        } => {
            let note_type: MidenNoteType = parse_note_type(note_type.to_string());
            let amount_as_u64: u64 = amount.parse::<u64>().map_err(|err| err.to_string())?;

            let faucet_id = AccountId::from_hex(faucet_id).map_err(|err| err.to_string())?;
            let fungible_asset =
                FungibleAsset::new(faucet_id, amount_as_u64).map_err(|err| err.to_string())?;
            let target_account_id =
                AccountId::from_hex(target_account_id).map_err(|err| err.to_string())?;

            Ok(TransactionTemplate::MintFungibleAsset(
                fungible_asset,
                target_account_id,
                note_type,
            ))
        },
        TransactionType::ConsumeNotes { account_id, list_of_notes } => {

            let mut note_ids = Vec::new();
            for note_id in list_of_notes.iter() {
                let note_record = get_note_with_id_prefix(client, note_id).await
                                .unwrap();
                note_ids.push(note_record.id());
            }
            let list_of_notes = note_ids; // now contains Vec<NoteId>

            let account_id = AccountId::from_hex(account_id).map_err(|err| err.to_string())?;

            Ok(TransactionTemplate::ConsumeNotes(account_id, list_of_notes))
        },
    }
}

/// Returns all client's notes whose ID starts with `note_id_prefix`
///
/// # Errors
///
/// - Returns [NoteIdPrefixFetchError::NoMatch] if we were unable to find any note where
/// `note_id_prefix` is a prefix of its id.
/// - Returns [NoteIdPrefixFetchError::MultipleMatches] if there were more than one note found
/// where `note_id_prefix` is a prefix of its id.
pub(crate) async fn get_note_with_id_prefix<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &Client<N, R, S>,
    note_id_prefix: &str,
) -> Result<InputNoteRecord, NoteIdPrefixFetchError> {
    // log the note_id_prefix
    let input_note_records = client
        .get_input_notes(NoteFilter::All).await
        .map_err(|err| NoteIdPrefixFetchError::NoMatch(note_id_prefix.to_string()))?
        .into_iter()
        .filter(|note_record| note_record.id().to_hex().starts_with(note_id_prefix))
        .collect::<Vec<_>>();

    if input_note_records.is_empty() {
        return Err(NoteIdPrefixFetchError::NoMatch(note_id_prefix.to_string()));
    }
    if input_note_records.len() > 1 {
        let input_note_record_ids = input_note_records
            .iter()
            .map(|input_note_record| input_note_record.id())
            .collect::<Vec<_>>();
        tracing::error!(
            "Multiple notes found for the prefix {}: {:?}",
            note_id_prefix,
            input_note_record_ids
        );
        return Err(NoteIdPrefixFetchError::MultipleMatches(note_id_prefix.to_string()));
    }

    Ok(input_note_records[0].clone())
}
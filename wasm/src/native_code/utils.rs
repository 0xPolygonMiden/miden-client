use super::{
    rpc::NodeRpcClient, 
    Client, 
    store::Store // TODO: Add AuthInfo
};

// pub fn build_transaction_template<N: NodeRpcClient, S: Store>(
//     client: &Client<N, S>,
//     transaction_type: &TransactionType,
// ) -> Result<TransactionTemplate, String> {
//     match transaction_type {
//         TransactionType::P2ID {
//             sender_account_id,
//             target_account_id,
//             faucet_id,
//             amount,
//         } => {
//             let faucet_id = AccountId::from_hex(faucet_id).map_err(|err| err.to_string())?;
//             let fungible_asset =
//                 FungibleAsset::new(faucet_id, *amount).map_err(|err| err.to_string())?.into();
//             let sender_account_id =
//                 AccountId::from_hex(sender_account_id).map_err(|err| err.to_string())?;
//             let target_account_id =
//                 AccountId::from_hex(target_account_id).map_err(|err| err.to_string())?;
//             let payment_transaction =
//                 PaymentTransactionData::new(fungible_asset, sender_account_id, target_account_id);

//             Ok(TransactionTemplate::PayToId(payment_transaction))
//         },
//         TransactionType::P2IDR => {
//             todo!()
//         },
//         TransactionType::Mint {
//             faucet_id,
//             target_account_id,
//             amount,
//         } => {
//             let faucet_id = AccountId::from_hex(faucet_id).map_err(|err| err.to_string())?;
//             let fungible_asset =
//                 FungibleAsset::new(faucet_id, *amount).map_err(|err| err.to_string())?;
//             let target_account_id =
//                 AccountId::from_hex(target_account_id).map_err(|err| err.to_string())?;

//             Ok(TransactionTemplate::MintFungibleAsset {
//                 asset: fungible_asset,
//                 target_account_id,
//             })
//         },
//         TransactionType::ConsumeNotes {
//             account_id,
//             list_of_notes,
//         } => {
//             let list_of_notes = list_of_notes
//                 .iter()
//                 .map(|note_id| {
//                     get_note_with_id_prefix(client, note_id)
//                         .map(|note_record| note_record.note_id())
//                         .map_err(|err| err.to_string())
//                 })
//                 .collect::<Result<Vec<NoteId>, _>>()?;

//             let account_id = AccountId::from_hex(account_id).map_err(|err| err.to_string())?;

//             Ok(TransactionTemplate::ConsumeNotes(account_id, list_of_notes))
//         },
//     }
// }

// pub(crate) fn get_note_with_id_prefix<N: NodeRpcClient, S: Store>(
//     client: &Client<N, S>,
//     note_id_prefix: &str,
// ) -> Result<InputNoteRecord, NoteIdPrefixFetchError> {
//     let input_note_records = client
//         .get_input_notes(ClientNoteFilter::All)
//         .map_err(|err| {
//             tracing::error!("Error when fetching all notes from the store: {err}");
//             NoteIdPrefixFetchError::NoMatch(note_id_prefix.to_string())
//         })?
//         .into_iter()
//         .filter(|note_record| note_record.note_id().to_hex().starts_with(note_id_prefix))
//         .collect::<Vec<_>>();

//     if input_note_records.is_empty() {
//         return Err(NoteIdPrefixFetchError::NoMatch(note_id_prefix.to_string()));
//     }
//     if input_note_records.len() > 1 {
//         let input_note_record_ids = input_note_records
//             .iter()
//             .map(|input_note_record| input_note_record.note_id())
//             .collect::<Vec<_>>();
//         tracing::error!(
//             "Multiple notes found for the prefix {}: {:?}",
//             note_id_prefix,
//             input_note_record_ids
//         );
//         return Err(NoteIdPrefixFetchError::MultipleMatches(note_id_prefix.to_string()));
//     }

//     Ok(input_note_records[0].clone())
// }

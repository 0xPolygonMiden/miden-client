use miden_client::{
    client::{
        rpc::NodeRpcClient,
        transactions::{PaymentTransactionData, TransactionRecord, TransactionTemplate},
    },
    store::{Store, TransactionFilter},
};
use miden_objects::{accounts::AccountId, assets::FungibleAsset, notes::NoteId};
use tracing::info;

use super::{get_note_with_id_prefix, Client, Parser};
use crate::cli::create_dynamic_table;

#[derive(Clone, Debug, Parser)]
#[clap()]
pub enum TransactionType {
    /// Create a Pay To ID transaction.
    P2ID {
        sender_account_id: String,
        target_account_id: String,
        faucet_id: String,
        amount: u64,
    },
    /// Mint `amount` tokens from the specified fungible faucet (corresponding to `faucet_id`). The created note can then be then consumed by
    /// `target_account_id`.
    Mint {
        target_account_id: String,
        faucet_id: String,
        amount: u64,
    },
    /// Create a Pay To ID with Recall transaction.
    P2IDR {
        sender_account_id: String,
        target_account_id: String,
        faucet_id: String,
        amount: u64,
        recall_height: u32,
    },
    /// Consume with the account corresponding to `account_id` all of the notes from `list_of_notes`.
    ConsumeNotes {
        account_id: String,
        /// A list of note IDs or the hex prefixes of their corresponding IDs
        list_of_notes: Vec<String>,
    },
}

#[derive(Debug, Parser, Clone)]
#[clap(about = "Execute and view transactions")]
pub enum Transaction {
    /// List currently tracked transactions
    #[clap(short_flag = 'l')]
    List,
    /// Execute a transaction, prove and submit it to the node. Once submitted, it
    /// gets tracked by the client
    #[clap(short_flag = 'n')]
    New {
        #[clap(subcommand)]
        transaction_type: TransactionType,
    },
}

impl Transaction {
    pub async fn execute<N: NodeRpcClient, S: Store>(
        &self,
        mut client: Client<N, S>,
    ) -> Result<(), String> {
        match self {
            Transaction::List => {
                list_transactions(client)?;
            },
            Transaction::New { transaction_type } => {
                new_transaction(&mut client, transaction_type).await?;
            },
        }
        Ok(())
    }
}

// NEW TRANSACTION
// ================================================================================================
async fn new_transaction<N: NodeRpcClient, S: Store>(
    client: &mut Client<N, S>,
    transaction_type: &TransactionType,
) -> Result<(), String> {
    let transaction_template: TransactionTemplate =
        build_transaction_template(client, transaction_type)?;

    let transaction_execution_result = client.new_transaction(transaction_template.clone())?;

    info!("Executed transaction, proving and then submitting...");

    client.send_transaction(transaction_execution_result).await?;

    Ok(())
}

/// Builds a [TransactionTemplate] based on the transaction type provided via cli args
///
/// For [TransactionTemplate::ConsumeNotes], it'll try to find the corresponding notes by using the
/// provided IDs as prefixes
fn build_transaction_template<N: NodeRpcClient, S: Store>(
    client: &Client<N, S>,
    transaction_type: &TransactionType,
) -> Result<TransactionTemplate, String> {
    match transaction_type {
        TransactionType::P2ID {
            sender_account_id,
            target_account_id,
            faucet_id,
            amount,
        } => {
            let faucet_id = AccountId::from_hex(faucet_id).map_err(|err| err.to_string())?;
            let fungible_asset =
                FungibleAsset::new(faucet_id, *amount).map_err(|err| err.to_string())?.into();
            let sender_account_id =
                AccountId::from_hex(sender_account_id).map_err(|err| err.to_string())?;
            let target_account_id =
                AccountId::from_hex(target_account_id).map_err(|err| err.to_string())?;
            let payment_transaction =
                PaymentTransactionData::new(fungible_asset, sender_account_id, target_account_id);

            Ok(TransactionTemplate::PayToId(payment_transaction))
        }
        TransactionType::P2IDR {
            sender_account_id,
            target_account_id,
            faucet_id,
            amount,
            recall_height,
        } => {
            let faucet_id = AccountId::from_hex(faucet_id).map_err(|err| err.to_string())?;
            let fungible_asset = FungibleAsset::new(faucet_id, *amount)
                .map_err(|err| err.to_string())?
                .into();
            let sender_account_id =
                AccountId::from_hex(sender_account_id).map_err(|err| err.to_string())?;
            let target_account_id =
                AccountId::from_hex(target_account_id).map_err(|err| err.to_string())?;
            let payment_transaction =
                PaymentTransactionData::new(fungible_asset, sender_account_id, target_account_id);

            Ok(TransactionTemplate::PayToIdWithRecall(
                payment_transaction,
                *recall_height,
            ))
        },
        TransactionType::Mint {
            faucet_id,
            target_account_id,
            amount,
        } => {
            let faucet_id = AccountId::from_hex(faucet_id).map_err(|err| err.to_string())?;
            let fungible_asset =
                FungibleAsset::new(faucet_id, *amount).map_err(|err| err.to_string())?;
            let target_account_id =
                AccountId::from_hex(target_account_id).map_err(|err| err.to_string())?;

            Ok(TransactionTemplate::MintFungibleAsset {
                asset: fungible_asset,
                target_account_id,
            })
        },
        TransactionType::ConsumeNotes {
            account_id,
            list_of_notes,
        } => {
            let list_of_notes = list_of_notes
                .iter()
                .map(|note_id| {
                    get_note_with_id_prefix(client, note_id)
                        .map(|note_record| note_record.note_id())
                        .map_err(|err| err.to_string())
                })
                .collect::<Result<Vec<NoteId>, _>>()?;

            let account_id = AccountId::from_hex(account_id).map_err(|err| err.to_string())?;

            Ok(TransactionTemplate::ConsumeNotes(account_id, list_of_notes))
        },
    }
}

// LIST TRANSACTIONS
// ================================================================================================
fn list_transactions<N: NodeRpcClient, S: Store>(client: Client<N, S>) -> Result<(), String> {
    let transactions = client.get_transactions(TransactionFilter::All)?;
    print_transactions_summary(&transactions);
    Ok(())
}

// HELPERS
// ================================================================================================
fn print_transactions_summary<'a, I>(executed_transactions: I)
where
    I: IntoIterator<Item = &'a TransactionRecord>,
{
    let mut table = create_dynamic_table(&[
        "ID",
        "Status",
        "Account ID",
        "Script Hash",
        "Input Notes Count",
        "Output Notes Count",
    ]);

    for tx in executed_transactions {
        table.add_row(vec![
            tx.id.to_string(),
            tx.transaction_status.to_string(),
            tx.account_id.to_string(),
            tx.transaction_script
                .as_ref()
                .map(|x| x.hash().to_string())
                .unwrap_or("-".to_string()),
            tx.input_note_nullifiers.len().to_string(),
            tx.output_notes.num_notes().to_string(),
        ]);
    }

    println!("{table}");
}

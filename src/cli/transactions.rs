use miden_client::{
    client::{
        rpc::NodeRpcClient,
        transactions::{PaymentTransactionData, TransactionRecord, TransactionTemplate},
    },
    store::{Store, TransactionFilter},
};

use miden_tx::DataStore;
use objects::{accounts::AccountId, assets::FungibleAsset, notes::NoteId};
use tracing::info;

use crate::cli::create_dynamic_table;

use super::{Client, Parser};

#[derive(Clone, Debug, Parser)]
#[clap()]
pub enum TransactionType {
    P2ID {
        sender_account_id: String,
        target_account_id: String,
        faucet_id: String,
        amount: u64,
    },
    Mint {
        target_account_id: String,
        faucet_id: String,
        amount: u64,
    },
    P2IDR,
    ConsumeNotes {
        account_id: String,
        list_of_notes: Vec<String>,
    },
}

impl TryInto<TransactionTemplate> for &TransactionType {
    type Error = String;

    fn try_into(self) -> Result<TransactionTemplate, Self::Error> {
        match self {
            TransactionType::P2ID {
                sender_account_id,
                target_account_id,
                faucet_id,
                amount,
            } => {
                let faucet_id = AccountId::from_hex(faucet_id).map_err(|err| err.to_string())?;
                let fungible_asset = FungibleAsset::new(faucet_id, *amount)
                    .map_err(|err| err.to_string())?
                    .into();
                let sender_account_id =
                    AccountId::from_hex(sender_account_id).map_err(|err| err.to_string())?;
                let target_account_id =
                    AccountId::from_hex(target_account_id).map_err(|err| err.to_string())?;
                let payment_transaction = PaymentTransactionData::new(
                    fungible_asset,
                    sender_account_id,
                    target_account_id,
                );

                Ok(TransactionTemplate::PayToId(payment_transaction))
            }
            TransactionType::P2IDR => {
                todo!()
            }
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
            }
            TransactionType::ConsumeNotes {
                account_id,
                list_of_notes,
            } => {
                let list_of_notes = list_of_notes
                    .iter()
                    .map(|n| NoteId::try_from_hex(n).map_err(|err| err.to_string()))
                    .collect::<Result<Vec<NoteId>, _>>()?;

                let account_id = AccountId::from_hex(account_id).map_err(|err| err.to_string())?;

                Ok(TransactionTemplate::ConsumeNotes(account_id, list_of_notes))
            }
        }
    }
}

#[derive(Debug, Parser, Clone)]
#[clap(about = "Execute and view transactions")]
pub enum Transaction {
    /// List transactions
    #[clap(short_flag = 'l')]
    List,
    /// Execute a transaction, prove and submit it to the node
    #[clap(short_flag = 'n')]
    New {
        #[clap(subcommand)]
        transaction_type: TransactionType,
    },
}

impl Transaction {
    pub async fn execute<N: NodeRpcClient, S: Store, D: DataStore>(
        &self,
        mut client: Client<N, S, D>,
    ) -> Result<(), String> {
        match self {
            Transaction::List => {
                list_transactions(client)?;
            }
            Transaction::New { transaction_type } => {
                let transaction_template: TransactionTemplate = transaction_type.try_into()?;

                let transaction_execution_result =
                    client.new_transaction(transaction_template.clone())?;

                info!("Executed transaction, proving and then submitting...");

                client
                    .send_transaction(transaction_execution_result)
                    .await?
            }
        }
        Ok(())
    }
}

// LIST TRANSACTIONS
// ================================================================================================
fn list_transactions<N: NodeRpcClient, S: Store, D: DataStore>(
    client: Client<N, S, D>,
) -> Result<(), String> {
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

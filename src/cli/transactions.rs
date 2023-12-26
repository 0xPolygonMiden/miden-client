use comfy_table::presets;
use comfy_table::Attribute;
use comfy_table::Cell;
use comfy_table::ContentArrangement;
use comfy_table::Table;

use miden_client::client::transactions::PaymentTransactionData;
use miden_client::client::transactions::TransactionStub;
use miden_client::client::transactions::TransactionTemplate;
use objects::accounts::AccountId;
use objects::assets::FungibleAsset;

use super::Client;
use super::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(about = "View transactions")]
pub enum Transaction {
    /// List transactions
    #[clap(short_flag = 'l')]
    List {
        /// List only pending transactions
        #[clap(short, long, default_value_t = false)]
        pending: bool,
    },
    /// Execute a transaction, prove and submit it to the node
    #[clap(short_flag = 'n')]
    New {
        #[clap(subcommand)]
        transaction_type: TransactionType,
    },
}

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
        tag: u64,
        faucet_id: String,
        amount: u64,
    },
    P2IDR,
}

impl Transaction {
    pub async fn execute(&self, mut client: Client) -> Result<(), String> {
        match self {
            Transaction::List { pending } => {
                list_transactions(client, *pending)?;
            }
            Transaction::New { transaction_type } => match transaction_type {
                TransactionType::P2ID {
                    sender_account_id,
                    target_account_id,
                    faucet_id,
                    amount,
                } => {
                    let faucet_id =
                        AccountId::from_hex(faucet_id).map_err(|err| err.to_string())?;
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
                    let transaction_execution_result = client
                        .new_transaction(TransactionTemplate::PayToId(payment_transaction))
                        .map_err(|err| err.to_string())?;

                    client
                        .send_transaction(transaction_execution_result)
                        .await
                        .map_err(|err| err.to_string())?;
                }
                TransactionType::P2IDR => {
                    todo!()
                }
                TransactionType::Mint {
                    faucet_id,
                    tag,
                    target_account_id,
                    amount,
                } => {
                    let faucet_id =
                        AccountId::from_hex(faucet_id).map_err(|err| err.to_string())?;
                    let fungible_asset = FungibleAsset::new(faucet_id, *amount)
                        .map_err(|err| err.to_string())?;
                    let target_account_id =
                        AccountId::from_hex(target_account_id).map_err(|err| err.to_string())?;
                    let transaction_template = TransactionTemplate::MintFungibleAsset {
                        asset: fungible_asset,
                        tag: *tag,
                        target_account_id,
                    };

                    let transaction_execution_result = client
                        .new_transaction(transaction_template)
                        .map_err(|err| err.to_string())?;

                    client
                        .send_transaction(transaction_execution_result)
                        .await
                        .map_err(|err| err.to_string())?;
                }
            },
        }
        Ok(())
    }
}

// LIST TRANSACTIONS
// ================================================================================================
fn list_transactions(client: Client, only_show_pending: bool) -> Result<(), String> {
    let transactions = client.get_transactions().map_err(|err| err.to_string())?;
    print_transactions_summary(&transactions, only_show_pending);
    Ok(())
}

// HELPERS
// ================================================================================================
fn print_transactions_summary<'a, I>(executed_transactions: I, _only_show_pending: bool)
where
    I: IntoIterator<Item = &'a TransactionStub>,
{
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_HORIZONTAL_ONLY)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth);

    table
        .load_preset(presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec![
            Cell::new("account id").add_attribute(Attribute::Bold),
            Cell::new("script hash").add_attribute(Attribute::Bold),
            Cell::new("committed").add_attribute(Attribute::Bold),
            Cell::new("block number").add_attribute(Attribute::Bold),
            Cell::new("input notes count").add_attribute(Attribute::Bold),
        ]);

    for tx in executed_transactions {
        table.add_row(vec![
            tx.account_id.to_string(),
            tx.transaction_script
                .as_ref()
                .map(|x| x.hash().to_string())
                .unwrap_or("-".to_string()),
            tx.committed.to_string(),
            tx.block_num.to_string(),
            tx.input_note_nullifiers.len().to_string(),
        ]);
    }

    println!("{table}");
}

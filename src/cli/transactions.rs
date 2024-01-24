use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};

use miden_client::{
    client::transactions::{PaymentTransactionData, TransactionStub, TransactionTemplate},
    store::transactions::TransactionFilter,
};
use objects::{accounts::AccountId, assets::FungibleAsset};

use super::{Client, Parser};

#[derive(Debug, Parser, Clone)]
#[clap(about = "View transactions")]
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
}

impl Transaction {
    pub async fn execute(&self, mut client: Client) -> Result<(), String> {
        match self {
            Transaction::List => {
                list_transactions(client)?;
            }
            Transaction::New { transaction_type } => {
                let transaction_template = match transaction_type {
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
                        let sender_account_id = AccountId::from_hex(sender_account_id)
                            .map_err(|err| err.to_string())?;
                        let target_account_id = AccountId::from_hex(target_account_id)
                            .map_err(|err| err.to_string())?;
                        let payment_transaction = PaymentTransactionData::new(
                            fungible_asset,
                            sender_account_id,
                            target_account_id,
                        );

                        TransactionTemplate::PayToId(payment_transaction)
                    }
                    TransactionType::P2IDR => {
                        todo!()
                    }
                    TransactionType::Mint {
                        faucet_id,
                        target_account_id,
                        amount,
                    } => {
                        let faucet_id =
                            AccountId::from_hex(faucet_id).map_err(|err| err.to_string())?;
                        let fungible_asset = FungibleAsset::new(faucet_id, *amount)
                            .map_err(|err| err.to_string())?;
                        let target_account_id = AccountId::from_hex(target_account_id)
                            .map_err(|err| err.to_string())?;
                        TransactionTemplate::MintFungibleAsset {
                            asset: fungible_asset,
                            target_account_id,
                        }
                    }
                };
                let transaction_execution_result = client
                    .new_transaction(transaction_template)
                    .map_err(|err| err.to_string())?;
                println!("Executed transaction, proving and then submitting...");
                client
                    .send_transaction(transaction_execution_result)
                    .await
                    .map_err(|err| err.to_string())?;
            }
        }
        Ok(())
    }
}

// LIST TRANSACTIONS
// ================================================================================================
fn list_transactions(client: Client) -> Result<(), String> {
    let transactions = client
        .get_transactions(TransactionFilter::All)
        .map_err(|err| err.to_string())?;
    print_transactions_summary(&transactions);
    Ok(())
}

// HELPERS
// ================================================================================================
fn print_transactions_summary<'a, I>(executed_transactions: I)
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
            Cell::new("output notes count").add_attribute(Attribute::Bold),
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
            tx.output_notes.num_notes().to_string(),
        ]);
    }

    println!("{table}");
}

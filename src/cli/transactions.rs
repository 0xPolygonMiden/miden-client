use comfy_table::presets;
use comfy_table::Attribute;
use comfy_table::Cell;
use comfy_table::ContentArrangement;
use comfy_table::Table;
use miden_client::TransactionStub;

use super::Client;
use super::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(about = "View transactions")]
pub enum Transaction {
    /// Execute a transaction, prove and submit it to the node
    #[clap(short_flag = 'n')]
    New {
        #[clap(subcommand)]
        transaction_type: TransactionType,
    },
    /// List transactions
    #[clap(short_flag = 'l')]
    List {
        /// List only pending transactions
        #[clap(short, long, default_value_t = false)]
        pending: bool,
    },
}

#[derive(Clone, Debug, Parser)]
#[clap()]
pub enum TransactionType {
    P2ID {
        sender_account_id: String,
        target_account_id: String,
        faucet_id: String,
    },
    P2ID2,
}

impl Transaction {
    pub fn execute(&self, client: Client) -> Result<(), String> {
        match self {
            Transaction::List { pending } => {
                list_transactions(client, *pending)?;
            }
            Transaction::New { .. } => todo!(),
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
            Cell::new("script inputs").add_attribute(Attribute::Bold),
        ]);

    for tx in executed_transactions {
        table.add_row(vec![
            tx.account_id.to_string(),
            tx.script_hash
                .map(|hash| hash.to_string())
                .unwrap_or("-".into()),
            tx.committed.to_string(),
            tx.block_ref.to_string(),
            serde_json::to_string(&tx.script_inputs).unwrap_or("-".into()),
        ]);
    }

    println!("{table}");
}

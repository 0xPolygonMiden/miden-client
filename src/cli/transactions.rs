use comfy_table::presets;
use comfy_table::Attribute;
use comfy_table::Cell;
use comfy_table::ContentArrangement;
use comfy_table::Table;
use miden_client::client::transactions::TransactionStub;

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
}

impl Transaction {
    pub fn execute(&self, client: Client) -> Result<(), String> {
        match self {
            Transaction::List { pending } => {
                list_transactions(client, *pending)?;
            }
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
        ]);
    }

    println!("{table}");
}

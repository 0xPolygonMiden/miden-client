use miden_client::{Client, store::TransactionFilter, transaction::TransactionRecord};

use crate::{Parser, create_dynamic_table, errors::CliError};

#[derive(Default, Debug, Parser, Clone)]
#[clap(about = "Manage and view transactions. Defaults to `list` command.")]
pub struct TransactionCmd {
    /// List currently tracked transactions.
    #[clap(short, long, group = "action")]
    list: bool,
}

impl TransactionCmd {
    pub async fn execute(&self, client: Client) -> Result<(), CliError> {
        list_transactions(client).await?;
        Ok(())
    }
}

// LIST TRANSACTIONS
// ================================================================================================
async fn list_transactions(client: Client) -> Result<(), CliError> {
    let transactions = client.get_transactions(TransactionFilter::All).await?;
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
            tx.transaction_script.as_ref().map_or("-".to_string(), |x| x.hash().to_string()),
            tx.input_note_nullifiers.len().to_string(),
            tx.output_notes.num_notes().to_string(),
        ]);
    }

    println!("{table}");
}

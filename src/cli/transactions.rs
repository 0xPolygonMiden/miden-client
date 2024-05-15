use miden_client::{
    client::{rpc::NodeRpcClient, transactions::TransactionRecord},
    store::{Store, TransactionFilter},
};
use miden_objects::crypto::rand::FeltRng;
use miden_tx::TransactionAuthenticator;

use super::{Client, Parser};
use crate::cli::create_dynamic_table;

#[derive(Default, Debug, Parser, Clone)]
#[clap(about = "Manage and view transactions. Defaults to `list` command.")]
pub struct TransactionCmd {
    /// List currently tracked transactions
    #[clap(short, long, group = "action")]
    list: bool,
}

impl TransactionCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        list_transactions(client)?;
        Ok(())
    }
}

// LIST TRANSACTIONS
// ================================================================================================
fn list_transactions<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: Client<N, R, S, A>,
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

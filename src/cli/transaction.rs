use super::Client;
use super::Parser;

use miden_client::ExecutedTransactionStub;
use miden_tx::DataStore;

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
    pub fn execute(&self, client: Client<impl DataStore>) -> Result<(), String> {
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
fn list_transactions(
    client: Client<impl DataStore>,
    only_show_pending: bool,
) -> Result<(), String> {
    let transactions = client.get_transactions().map_err(|err| err.to_string())?;
    print_transactions_summary(&transactions, only_show_pending);
    Ok(())
}

// HELPERS
// ================================================================================================
fn print_transactions_summary<'a, I>(executed_transactions: I, only_show_pending: bool)
where
    I: IntoIterator<Item = &'a ExecutedTransactionStub>,
{
    println!("{}", "-".repeat(240));
    println!(
        "{0: <30} | {1: <30} | {2: <30} | {3: <30} | {4: <15}",
        "account id", "script hash", "committed", "initial block", "inputs",
    );
    println!("{}", "-".repeat(240));

    for tx in executed_transactions {
        if only_show_pending && tx.committed {
            continue;
        }

        println!(
            "{0: <30} | {1: <30} | {2: <30} | {3: <30} | {4: <15}",
            tx.account_id,
            tx.script_hash
                .map(|hash| hash.to_string())
                .unwrap_or("-".into()),
            tx.committed,
            tx.block_num,
            serde_json::to_string(&tx.script_inputs).unwrap()
        );
    }
    println!("{}", "-".repeat(240));
}

use std::path::PathBuf;

use clap::Parser;
use miden_client::{client::Client, config::ClientConfig};

mod account;
mod input_notes;
mod sync_state;
mod transactions;

/// Root CLI struct
#[derive(Parser, Debug)]
#[clap(
    name = "Miden",
    about = "Miden Client",
    version,
    rename_all = "kebab-case"
)]
pub struct Cli {
    #[clap(subcommand)]
    action: Command,
}

/// CLI actions
#[derive(Debug, Parser)]
pub enum Command {
    #[clap(subcommand)]
    Account(account::AccountCmd),
    #[clap(subcommand)]
    InputNotes(input_notes::InputNotes),
    #[clap(subcommand)]
    SyncState(sync_state::SyncStateCmd),
    #[clap(subcommand)]
    Transaction(transactions::Transaction),
    #[cfg(feature = "mock")]
    /// Insert mock data into the client. This is optional because it takes a few seconds
    MockData {
        #[clap(short, long)]
        transaction: bool,
    },
    //#[cfg(feature = "testing")]
    /// Insert data from node's genesis file
    LoadGenesis {
        /// The directory that contains the files generated from the node: genesis.dat, faucet.fsk
        /// and wallet.fs and the accounts directory containing account{X}.mac files, one for each
        /// account
        #[clap(short, long)]
        genesis_path: PathBuf,

        /// The indices of accounts to import, if account indices contains the value `i`, then it
        /// will import account at "{genesis_path}/accounts/account{i}.mac". If not provided takes
        /// all files possible
        #[clap(short, long, value_delimiter = ' ', num_args=1..)]
        account_indices: Option<Vec<usize>>,
    },
}

/// CLI entry point
impl Cli {
    pub async fn execute(&self) -> Result<(), String> {
        // create a client
        let client = Client::new(ClientConfig::default())
            .await
            .map_err(|err| err.to_string())?;

        // execute cli command
        match &self.action {
            Command::Account(account) => account.execute(client),
            Command::InputNotes(notes) => notes.execute(client),
            Command::SyncState(tags) => tags.execute(client).await,
            Command::Transaction(transaction) => transaction.execute(client).await,
            #[cfg(feature = "mock")]
            Command::MockData { transaction } => {
                let mut client = client;
                miden_client::mock::insert_mock_data(&mut client);
                if *transaction {
                    miden_client::mock::create_mock_transaction(&mut client).await;
                }
                Ok(())
            }
            Command::LoadGenesis {
                genesis_path,
                account_indices,
            } => {
                let mut client = client;
                client.load_genesis_data(genesis_path, account_indices.clone())
            }
        }
    }
}

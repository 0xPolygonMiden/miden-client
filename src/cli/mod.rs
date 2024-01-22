use clap::Parser;
use crypto::utils::Deserializable;
use miden_client::{client::Client, config::ClientConfig};
use objects::accounts::AccountData;
use std::{
    fs,
    path::{Path, PathBuf},
};

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
    LoadAccounts {
        /// The directory that contains account data files (i.e., .mac filed), one file for each
        /// account.
        #[clap(short, long)]
        accounts_path: PathBuf,
    },
    LoadAccount {
        /// The path to the account data file.
        #[clap(short, long)]
        account_path: PathBuf,
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
            Command::LoadAccounts {
                accounts_path,
            } => {
                let mut client = client;
                load_accounts_data(&mut client, accounts_path)
            }
            Command::LoadAccount {
                account_path,
            } => {
                let mut client = client;
                load_account(&mut client, account_path)
            }
        }
    }
}

fn load_accounts_data(
    client: &mut Client,
    path: &Path,
) -> Result<(), String> {
    if !PathBuf::new().join(path).exists() {
        return Err("The specified path does not exist".to_string());
    }

    let mac_account_files = fs::read_dir(path)
        .unwrap()
        .filter_map(|file| file.ok())
        .filter(|file| file.path().extension().map_or(false, |ext| ext == "mac"));

    for file in mac_account_files {
        load_account(client, &file.path())?;
    }

    Ok(())
}

fn load_account(client: &mut Client, account_data_path: &PathBuf) -> Result<(), String> {
    let account_data_file_contents =
        fs::read(account_data_path).map_err(|err| err.to_string())?;
    let account_data = AccountData::read_from_bytes(&account_data_file_contents)
        .map_err(|err| err.to_string())?;

    client.import_account(account_data)?;

    Ok(())
}

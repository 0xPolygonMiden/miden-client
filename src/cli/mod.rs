use std::path::Path;

use clap::Parser;
use figment::{
    providers::{Format, Toml},
    Figment,
};
use miden_client::{client::Client, config::ClientConfig};

mod account;
mod input_notes;
mod sync_state;
mod transactions;

/// Config file name
const CLIENT_CONFIG_FILE_NAME: &str = "miden.toml";

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
    #[clap(subcommand, name = "tx")]
    #[clap(visible_alias="transaction")]
    Transaction(transactions::Transaction),
    #[cfg(feature = "mock")]
    /// Insert mock data into the client. This is optional because it takes a few seconds
    MockData {
        #[clap(short, long)]
        transaction: bool,
    },
}

/// CLI entry point
impl Cli {
    pub async fn execute(&self) -> Result<(), String> {
        // Create the client
        let mut current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
        current_dir.push(CLIENT_CONFIG_FILE_NAME);
        println!("path {:?}", current_dir);

        let client_config = load_config(current_dir.as_path())?;
        let client = Client::new(client_config).map_err(|err| err.to_string())?;

        // Execute cli command
        match &self.action {
            Command::Account(account) => account.execute(client),
            Command::InputNotes(notes) => notes.execute(client),
            Command::SyncState(tags) => tags.execute(client).await,
            Command::Transaction(transaction) => transaction.execute(client).await,
            #[cfg(feature = "mock")]
            Command::MockData { transaction } => {
                let mut client = client;
                miden_client::mock::insert_mock_data(&mut client).await;
                if *transaction {
                    miden_client::mock::create_mock_transaction(&mut client).await;
                }
                Ok(())
            }
        }
    }
}

/// Loads the client configuration.
///
/// This function will look for the configuration file at the provided path. If the path is
/// relative, searches in parent directories all the way to the root as well.
pub fn load_config(config_file: &Path) -> Result<ClientConfig, String> {
    Figment::from(Toml::file(config_file))
        .extract()
        .map_err(|err| {
            format!(
                "Failed to load {} config file: {err}",
                config_file.display()
            )
        })
}

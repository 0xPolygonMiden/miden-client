use std::path::Path;

use clap::Parser;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
use figment::{
    providers::{Format, Toml},
    Figment,
};
use miden_client::{
    client::{rpc::NodeRpcClient, Client},
    config::ClientConfig,
};

#[cfg(feature = "mock")]
use miden_client::mock::MockDataStore;
#[cfg(feature = "mock")]
use miden_client::mock::MockRpcApi;

#[cfg(not(feature = "mock"))]
use miden_client::client::rpc::TonicRpcClient;
#[cfg(not(feature = "mock"))]
use miden_client::store::data_store::SqliteDataStore;

mod account;
mod info;
mod input_notes;
mod sync;
mod tags;
mod transactions;

/// Config file name
const CLIENT_CONFIG_FILE_NAME: &str = "miden-client.toml";

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
    /// Sync this client with the latest state of the Miden network.
    Sync,
    /// View a summary of the current client state
    Info,
    #[clap(subcommand)]
    Tags(tags::TagsCmd),
    #[clap(subcommand, name = "tx")]
    #[clap(visible_alias = "transaction")]
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

        let client_config = load_config(current_dir.as_path())?;
        let rpc_endpoint = client_config.rpc.endpoint.to_string();

        #[cfg(not(feature = "mock"))]
        let client: Client<TonicRpcClient, SqliteDataStore> = {
            use miden_client::{errors::ClientError, store::Store};

            let store = Store::new((&client_config).into()).map_err(ClientError::StoreError)?;
            Client::new(
                client_config,
                TonicRpcClient::new(&rpc_endpoint),
                SqliteDataStore::new(store),
            )?
        };

        #[cfg(feature = "mock")]
        let client: Client<MockRpcApi, MockDataStore> = Client::new(
            client_config,
            MockRpcApi::new(&rpc_endpoint),
            MockDataStore::new(),
        )?;

        // Execute cli command
        match &self.action {
            Command::Account(account) => account.execute(client),
            Command::Info => info::print_client_info(&client),
            Command::InputNotes(notes) => notes.execute(client),
            Command::Sync => sync::sync_state(client).await,
            Command::Tags(tags) => tags.execute(client).await,
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

pub fn create_dynamic_table(headers: &[&str]) -> Table {
    let header_cells = headers
        .iter()
        .map(|header| Cell::new(header).add_attribute(Attribute::Bold))
        .collect::<Vec<_>>();

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(header_cells);

    table
}

use std::path::Path;

use clap::Parser;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
use figment::{
    providers::{Format, Toml},
    Figment,
};
use miden_client::{
    client::{
        get_random_coin,
        rpc::{NodeRpcClient, TonicRpcClient},
        Client,
    },
    config::ClientConfig,
    errors::{ClientError, NoteIdPrefixFetchError},
    store::{sqlite_store::SqliteStore, InputNoteRecord, NoteFilter as ClientNoteFilter, Store},
};
use miden_objects::crypto::rand::FeltRng;
#[cfg(not(feature = "mock"))]
use miden_objects::crypto::rand::RpoRandomCoin;

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
#[clap(name = "Miden", about = "Miden client", version, rename_all = "kebab-case")]
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
}

/// CLI entry point
impl Cli {
    pub async fn execute(&self) -> Result<(), String> {
        // Create the client
        let mut current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
        current_dir.push(CLIENT_CONFIG_FILE_NAME);

        let client_config = load_config(current_dir.as_path())?;
        let rpc_endpoint = client_config.rpc.endpoint.to_string();
        let store = SqliteStore::new((&client_config).into()).map_err(ClientError::StoreError)?;
        let rng = get_random_coin();
        let executor_store =
            miden_client::store::sqlite_store::SqliteStore::new((&client_config).into())
                .map_err(ClientError::StoreError)?;

        let client: Client<TonicRpcClient, RpoRandomCoin, SqliteStore> =
            Client::new(TonicRpcClient::new(&rpc_endpoint), rng, store, executor_store)?;

        // Execute cli command
        match &self.action {
            Command::Account(account) => account.execute(client),
            Command::Info => info::print_client_info(&client),
            Command::InputNotes(notes) => notes.execute(client),
            Command::Sync => sync::sync_state(client).await,
            Command::Tags(tags) => tags.execute(client).await,
            Command::Transaction(transaction) => transaction.execute(client).await,
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
        .map_err(|err| format!("Failed to load {} config file: {err}", config_file.display()))
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

/// Returns all client's notes whose ID starts with `note_id_prefix`
///
/// # Errors
///
/// - Returns [NoteIdPrefixFetchError::NoMatch] if we were unable to find any note where
/// `note_id_prefix` is a prefix of its id.
/// - Returns [NoteIdPrefixFetchError::MultipleMatches] if there were more than one note found
/// where `note_id_prefix` is a prefix of its id.
pub(crate) fn get_note_with_id_prefix<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &Client<N, R, S>,
    note_id_prefix: &str,
) -> Result<InputNoteRecord, NoteIdPrefixFetchError> {
    let input_note_records = client
        .get_input_notes(ClientNoteFilter::All)
        .map_err(|err| {
            tracing::error!("Error when fetching all notes from the store: {err}");
            NoteIdPrefixFetchError::NoMatch(note_id_prefix.to_string())
        })?
        .into_iter()
        .filter(|note_record| note_record.id().to_hex().starts_with(note_id_prefix))
        .collect::<Vec<_>>();

    if input_note_records.is_empty() {
        return Err(NoteIdPrefixFetchError::NoMatch(note_id_prefix.to_string()));
    }
    if input_note_records.len() > 1 {
        let input_note_record_ids = input_note_records
            .iter()
            .map(|input_note_record| input_note_record.id())
            .collect::<Vec<_>>();
        tracing::error!(
            "Multiple notes found for the prefix {}: {:?}",
            note_id_prefix,
            input_note_record_ids
        );
        return Err(NoteIdPrefixFetchError::MultipleMatches(note_id_prefix.to_string()));
    }

    Ok(input_note_records[0].clone())
}

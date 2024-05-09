use std::{env, fs::File, io::Write, path::Path};

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
    errors::{ClientError, IdPrefixFetchError},
    store::{
        sqlite_store::SqliteStore, InputNoteRecord, NoteFilter as ClientNoteFilter,
        OutputNoteRecord, Store,
    },
};
use miden_objects::{
    accounts::{AccountId, AccountStub},
    crypto::rand::{FeltRng, RpoRandomCoin},
};
use tracing::info;

use self::{
    account::AccountCmd, export::ExportCmd, import::ImportCmd, init::InitCmd, tags::TagsCmd,
};

mod account;
mod export;
mod import;
mod info;
mod init;
mod notes;
mod sync;
mod tags;
mod transactions;

/// Config file name
const CLIENT_CONFIG_FILE_NAME: &str = "miden-client.toml";

/// Client binary name
pub const CLIENT_BINARY_NAME: &str = "miden-client";

/// Root CLI struct
#[derive(Parser, Debug)]
#[clap(name = "Miden", about = "Miden client", version, rename_all = "kebab-case")]
pub struct Cli {
    #[clap(subcommand)]
    action: Command,

    /// Activates the executor's debug mode, which enables debug output for scripts
    /// that were compiled and executed with this mode.
    #[clap(short, long, default_value_t = false)]
    debug: bool,
}

/// CLI actions
#[derive(Debug, Parser)]
pub enum Command {
    Account {
        #[clap(subcommand)]
        cmd: Option<AccountCmd>,
    },
    #[clap(subcommand)]
    Import(ImportCmd),
    #[clap(subcommand)]
    Export(ExportCmd),
    Init(InitCmd),
    Notes {
        #[clap(subcommand)]
        cmd: Option<notes::Notes>,
    },
    /// Sync this client with the latest state of the Miden network.
    Sync,
    /// View a summary of the current client state
    Info,
    Tags {
        #[clap(subcommand)]
        cmd: Option<TagsCmd>,
    },
    #[clap(name = "tx")]
    #[clap(visible_alias = "transaction")]
    Transaction {
        #[clap(subcommand)]
        cmd: Option<transactions::Transaction>,
    },
}

/// CLI entry point
impl Cli {
    pub async fn execute(&self) -> Result<(), String> {
        let mut current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
        current_dir.push(CLIENT_CONFIG_FILE_NAME);

        // Check if it's an init command before anything else. When we run the init command for
        // the first time we won't have a config file and thus creating the store would not be
        // possible.
        if let Command::Init(init_cmd) = &self.action {
            init_cmd.execute(current_dir.clone())?;
            return Ok(());
        }

        // Define whether we want to use the executor's debug mode based on the env var and
        // the flag override

        let in_debug_mode = match env::var("MIDEN_DEBUG") {
            Ok(value) if value.to_lowercase() == "true" => true,
            _ => self.debug,
        };

        // Create the client
        let client_config = load_config(current_dir.as_path())?;
        let store = SqliteStore::new((&client_config).into()).map_err(ClientError::StoreError)?;
        let rng = get_random_coin();
        let _executor_store =
            miden_client::store::sqlite_store::SqliteStore::new((&client_config).into())
                .map_err(ClientError::StoreError)?;

        let client: Client<TonicRpcClient, RpoRandomCoin, SqliteStore> =
            Client::new(TonicRpcClient::new(&client_config.rpc), rng, store, in_debug_mode);

        // Execute CLI command
        match &self.action {
            Command::Account { cmd } => {
                let account = cmd.clone().unwrap_or_default();
                account.execute(client)
            },
            Command::Import(import) => import.execute(client).await,
            Command::Init(_) => Ok(()),
            Command::Info => info::print_client_info(&client),
            Command::Notes { cmd: notes_cmd } => {
                let notes_cmd = notes_cmd.clone().unwrap_or_default();
                notes_cmd.execute(client).await
            },
            Command::Sync => sync::sync_state(client).await,
            Command::Tags { cmd: tags_cmd } => {
                let tags_cmd = tags_cmd.clone().unwrap_or_default();
                tags_cmd.execute(client).await
            },
            Command::Transaction { cmd: transaction_cmd } => {
                let transaction_cmd = transaction_cmd.clone().unwrap_or_default();
                let default_account_id =
                    client_config.cli.and_then(|cli_conf| cli_conf.default_account_id);
                transaction_cmd.execute(client, default_account_id).await
            },
            Command::Export(cmd) => cmd.execute(client),
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

/// Returns the client input note whose ID starts with `note_id_prefix`
///
/// # Errors
///
/// - Returns [IdPrefixFetchError::NoMatch] if we were unable to find any note where
/// `note_id_prefix` is a prefix of its id.
/// - Returns [IdPrefixFetchError::MultipleMatches] if there were more than one note found
/// where `note_id_prefix` is a prefix of its id.
pub(crate) fn get_input_note_with_id_prefix<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &Client<N, R, S>,
    note_id_prefix: &str,
) -> Result<InputNoteRecord, IdPrefixFetchError> {
    let mut input_note_records = client
        .get_input_notes(ClientNoteFilter::All)
        .map_err(|err| {
            tracing::error!("Error when fetching all notes from the store: {err}");
            IdPrefixFetchError::NoMatch(format!("note ID prefix {note_id_prefix}").to_string())
        })?
        .into_iter()
        .filter(|note_record| note_record.id().to_hex().starts_with(note_id_prefix))
        .collect::<Vec<_>>();

    if input_note_records.is_empty() {
        return Err(IdPrefixFetchError::NoMatch(
            format!("note ID prefix {note_id_prefix}").to_string(),
        ));
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
        return Err(IdPrefixFetchError::MultipleMatches(
            format!("note ID prefix {note_id_prefix}").to_string(),
        ));
    }

    Ok(input_note_records
        .pop()
        .expect("input_note_records should always have one element"))
}

/// Returns the client output note whose ID starts with `note_id_prefix`
///
/// # Errors
///
/// - Returns [IdPrefixFetchError::NoMatch] if we were unable to find any note where
/// `note_id_prefix` is a prefix of its id.
/// - Returns [IdPrefixFetchError::MultipleMatches] if there were more than one note found
/// where `note_id_prefix` is a prefix of its id.
pub(crate) fn get_output_note_with_id_prefix<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &Client<N, R, S>,
    note_id_prefix: &str,
) -> Result<OutputNoteRecord, IdPrefixFetchError> {
    let mut output_note_records = client
        .get_output_notes(ClientNoteFilter::All)
        .map_err(|err| {
            tracing::error!("Error when fetching all notes from the store: {err}");
            IdPrefixFetchError::NoMatch(format!("note ID prefix {note_id_prefix}").to_string())
        })?
        .into_iter()
        .filter(|note_record| note_record.id().to_hex().starts_with(note_id_prefix))
        .collect::<Vec<_>>();

    if output_note_records.is_empty() {
        return Err(IdPrefixFetchError::NoMatch(
            format!("note ID prefix {note_id_prefix}").to_string(),
        ));
    }
    if output_note_records.len() > 1 {
        let output_note_record_ids = output_note_records
            .iter()
            .map(|input_note_record| input_note_record.id())
            .collect::<Vec<_>>();
        tracing::error!(
            "Multiple notes found for the prefix {}: {:?}",
            note_id_prefix,
            output_note_record_ids
        );
        return Err(IdPrefixFetchError::MultipleMatches(
            format!("note ID prefix {note_id_prefix}").to_string(),
        ));
    }

    Ok(output_note_records
        .pop()
        .expect("input_note_records should always have one element"))
}

/// Returns the client account whose ID starts with `account_id_prefix`
///
/// # Errors
///
/// - Returns [IdPrefixFetchError::NoMatch] if we were unable to find any account where
/// `account_id_prefix` is a prefix of its id.
/// - Returns [IdPrefixFetchError::MultipleMatches] if there were more than one account found
/// where `account_id_prefix` is a prefix of its id.
fn get_account_with_id_prefix<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &Client<N, R, S>,
    account_id_prefix: &str,
) -> Result<AccountStub, IdPrefixFetchError> {
    let mut accounts = client
        .get_account_stubs()
        .map_err(|err| {
            tracing::error!("Error when fetching all accounts from the store: {err}");
            IdPrefixFetchError::NoMatch(
                format!("account ID prefix {account_id_prefix}").to_string(),
            )
        })?
        .into_iter()
        .filter(|(account_stub, _)| account_stub.id().to_hex().starts_with(account_id_prefix))
        .map(|(acc, _)| acc)
        .collect::<Vec<_>>();

    if accounts.is_empty() {
        return Err(IdPrefixFetchError::NoMatch(
            format!("account ID prefix {account_id_prefix}").to_string(),
        ));
    }
    if accounts.len() > 1 {
        let account_ids = accounts.iter().map(|account_stub| account_stub.id()).collect::<Vec<_>>();
        tracing::error!(
            "Multiple accounts found for the prefix {}: {:?}",
            account_id_prefix,
            account_ids
        );
        return Err(IdPrefixFetchError::MultipleMatches(
            format!("account ID prefix {account_id_prefix}").to_string(),
        ));
    }

    Ok(accounts.pop().expect("account_ids should always have one element"))
}

/// Parses a user provided account id string and returns the corresponding `AccountId`
///
/// `account_id` can fall into two categories:
///
/// - it's a prefix of an account id of an account tracked by the client
/// - it's a full account id
///
/// # Errors
///
/// - Will return a `IdPrefixFetchError` if the provided account id string can't be parsed as an
/// `AccountId` and does not correspond to an account tracked by the client either.
pub(crate) fn parse_account_id<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: &Client<N, R, S>,
    account_id: &str,
) -> Result<AccountId, String> {
    if let Ok(account_id) = AccountId::from_hex(account_id) {
        return Ok(account_id);
    }

    let account_id = get_account_with_id_prefix(client, account_id)
        .map_err(|_err| "Input account ID {account_id} is neither a valid Account ID nor a prefix of a known Account ID")?
        .id();
    Ok(account_id)
}

pub(crate) fn update_config(config_path: &Path, client_config: ClientConfig) -> Result<(), String> {
    let config_as_toml_string = toml::to_string_pretty(&client_config)
        .map_err(|err| format!("error formatting config: {err}"))?;

    info!("Writing config file at: {:?}", config_path);
    let mut file_handle = File::options()
        .write(true)
        .truncate(true)
        .open(config_path)
        .map_err(|err| format!("error opening the file: {err}"))?;
    file_handle
        .write(config_as_toml_string.as_bytes())
        .map_err(|err| format!("error writing to file: {err}"))?;

    println!("Config updated successfully");
    Ok(())
}

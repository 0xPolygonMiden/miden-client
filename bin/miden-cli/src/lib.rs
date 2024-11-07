use std::{env, sync::Arc};

use clap::Parser;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
use miden_client::{
    accounts::AccountHeader,
    crypto::{FeltRng, RpoRandomCoin},
    rpc::TonicRpcClient,
    store::{
        sqlite_store::SqliteStore, NoteFilter as ClientNoteFilter, OutputNoteRecord,
        StoreAuthenticator,
    },
    transactions::{LocalTransactionProver, ProvingOptions},
    Client, ClientError, Felt, IdPrefixFetchError,
};
use rand::Rng;

mod commands;

use commands::{
    account::AccountCmd,
    export::ExportCmd,
    import::ImportCmd,
    init::InitCmd,
    new_account::{NewFaucetCmd, NewWalletCmd},
    new_transactions::{ConsumeNotesCmd, MintCmd, SendCmd, SwapCmd},
    notes::NotesCmd,
    sync::SyncCmd,
    tags::TagsCmd,
    transactions::TransactionCmd,
};

use self::utils::load_config_file;

mod config;
mod faucet_details_map;
mod info;
mod utils;

/// Config file name
const CLIENT_CONFIG_FILE_NAME: &str = "miden-client.toml";

/// Client binary name
pub const CLIENT_BINARY_NAME: &str = "miden";

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
    Account(AccountCmd),
    NewFaucet(NewFaucetCmd),
    NewWallet(NewWalletCmd),
    Import(ImportCmd),
    Export(ExportCmd),
    Init(InitCmd),
    Notes(NotesCmd),
    Sync(SyncCmd),
    /// View a summary of the current client state
    Info,
    Tags(TagsCmd),
    #[clap(name = "tx")]
    Transaction(TransactionCmd),
    Mint(MintCmd),
    Send(SendCmd),
    Swap(SwapCmd),
    ConsumeNotes(ConsumeNotesCmd),
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
        let (cli_config, _config_path) = load_config_file()?;
        let store = SqliteStore::new(&cli_config.store).await.map_err(ClientError::StoreError)?;
        let store = Arc::new(store);

        let mut rng = rand::thread_rng();
        let coin_seed: [u64; 4] = rng.gen();

        let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));
        let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);
        let tx_prover = LocalTransactionProver::new(ProvingOptions::default());

        let client = Client::new(
            Box::new(TonicRpcClient::new(&cli_config.rpc)),
            rng,
            store,
            Arc::new(authenticator),
            Arc::new(tx_prover),
            in_debug_mode,
        );

        // Execute CLI command
        match &self.action {
            Command::Account(account) => account.execute(client).await,
            Command::NewFaucet(new_faucet) => new_faucet.execute(client).await,
            Command::NewWallet(new_wallet) => new_wallet.execute(client).await,
            Command::Import(import) => import.execute(client).await,
            Command::Init(_) => Ok(()),
            Command::Info => info::print_client_info(&client, &cli_config).await,
            Command::Notes(notes) => notes.execute(client).await,
            Command::Sync(sync) => sync.execute(client).await,
            Command::Tags(tags) => tags.execute(client).await,
            Command::Transaction(transaction) => transaction.execute(client).await,
            Command::Export(cmd) => cmd.execute(client).await,
            Command::Mint(mint) => mint.execute(client).await,
            Command::Send(send) => send.execute(client).await,
            Command::Swap(swap) => swap.execute(client).await,
            Command::ConsumeNotes(consume_notes) => consume_notes.execute(client).await,
        }
    }
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

/// Returns the client output note whose ID starts with `note_id_prefix`
///
/// # Errors
///
/// - Returns [IdPrefixFetchError::NoMatch] if we were unable to find any note where
///   `note_id_prefix` is a prefix of its id.
/// - Returns [IdPrefixFetchError::MultipleMatches] if there were more than one note found where
///   `note_id_prefix` is a prefix of its id.
pub(crate) async fn get_output_note_with_id_prefix(
    client: &Client<impl FeltRng>,
    note_id_prefix: &str,
) -> Result<OutputNoteRecord, IdPrefixFetchError> {
    let mut output_note_records = client
        .get_output_notes(ClientNoteFilter::All)
        .await
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
///   `account_id_prefix` is a prefix of its id.
/// - Returns [IdPrefixFetchError::MultipleMatches] if there were more than one account found where
///   `account_id_prefix` is a prefix of its id.
async fn get_account_with_id_prefix(
    client: &Client<impl FeltRng>,
    account_id_prefix: &str,
) -> Result<AccountHeader, IdPrefixFetchError> {
    let mut accounts = client
        .get_account_headers()
        .await
        .map_err(|err| {
            tracing::error!("Error when fetching all accounts from the store: {err}");
            IdPrefixFetchError::NoMatch(
                format!("account ID prefix {account_id_prefix}").to_string(),
            )
        })?
        .into_iter()
        .filter(|(account_header, _)| account_header.id().to_hex().starts_with(account_id_prefix))
        .map(|(acc, _)| acc)
        .collect::<Vec<_>>();

    if accounts.is_empty() {
        return Err(IdPrefixFetchError::NoMatch(
            format!("account ID prefix {account_id_prefix}").to_string(),
        ));
    }
    if accounts.len() > 1 {
        let account_ids =
            accounts.iter().map(|account_header| account_header.id()).collect::<Vec<_>>();
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

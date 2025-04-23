use clap::Parser;
use comfy_table::{Cell, ContentArrangement, presets};
use miden_client::{
    Client, ZERO,
    account::{Account, AccountId, AccountType, StorageSlot},
    asset::Asset,
};
use miden_objects::PrettyPrint;

use crate::{
    CLIENT_BINARY_NAME,
    config::CliConfig,
    create_dynamic_table,
    errors::CliError,
    utils::{load_config_file, load_faucet_details_map, parse_account_id, update_config},
};

// ACCOUNT COMMAND
// ================================================================================================

/// View and manage accounts. Defaults to `list` command.
#[derive(Default, Debug, Clone, Parser)]
#[allow(clippy::option_option)]
pub struct AccountCmd {
    /// List all accounts monitored by this client (default action).
    #[clap(short, long, group = "action")]
    list: bool,
    /// Show details of the account for the specified ID or hex prefix.
    #[clap(short, long, group = "action", value_name = "ID")]
    show: Option<String>,
    /// When using --show, include the account code in the output.
    #[clap(long, requires = "show")]
    with_code: bool,
    /// Manages default account for transaction execution.
    ///
    /// If no ID is provided it will display the current default account ID.
    /// If "none" is provided it will remove the default account else it will set the default
    /// account to the provided ID.
    #[clap(short, long, group = "action", value_name = "ID")]
    default: Option<Option<String>>,
}

impl AccountCmd {
    pub async fn execute(&self, client: Client) -> Result<(), CliError> {
        let (cli_config, _) = load_config_file()?;
        match self {
            AccountCmd {
                list: false,
                show: Some(id),
                default: None,
                ..
            } => {
                let account_id = parse_account_id(&client, id).await?;
                show_account(client, account_id, &cli_config, self.with_code).await?;
            },
            AccountCmd {
                list: false,
                show: None,
                default: Some(id),
                ..
            } => {
                match id {
                    None => {
                        display_default_account_id()?;
                    },
                    Some(id) => {
                        let default_account = if id == "none" {
                            None
                        } else {
                            let account_id: AccountId = parse_account_id(&client, id).await?;

                            // Check whether we're tracking that account
                            let (account, _) = client.try_get_account_header(account_id).await?;

                            Some(account.id())
                        };

                        set_default_account(default_account)?;

                        if let Some(id) = default_account {
                            let id = id.to_hex();
                            println!("Setting default account to {id}...");
                        } else {
                            println!("Removing default account...");
                        }
                    },
                }
            },
            _ => {
                list_accounts(client).await?;
            },
        }
        Ok(())
    }
}

// LIST ACCOUNTS
// ================================================================================================

async fn list_accounts(client: Client) -> Result<(), CliError> {
    let accounts = client.get_account_headers().await?;

    let mut table =
        create_dynamic_table(&["Account ID", "Type", "Storage Mode", "Nonce", "Status"]);
    for (acc, _acc_seed) in &accounts {
        let status = client
            .get_account(acc.id())
            .await?
            .expect("Account should be in store")
            .status()
            .to_string();

        table.add_row(vec![
            acc.id().to_string(),
            account_type_display_name(&acc.id())?,
            acc.id().storage_mode().to_string(),
            acc.nonce().as_int().to_string(),
            status,
        ]);
    }

    println!("{table}");
    Ok(())
}

// SHOW ACCOUNT
// ================================================================================================

pub async fn show_account(
    client: Client,
    account_id: AccountId,
    cli_config: &CliConfig,
    with_code: bool,
) -> Result<(), CliError> {
    let account: Account = client
        .get_account(account_id)
        .await?
        .ok_or(CliError::Input(format!("Account with ID {account_id} not found")))?
        .into();

    print_summary_table(&account, cli_config)?;

    // Vault Table
    {
        let assets = account.vault().assets();
        let faucet_details_map = load_faucet_details_map()?;
        println!("Assets: ");

        let mut table = create_dynamic_table(&["Asset Type", "Faucet", "Amount"]);
        for asset in assets {
            let (asset_type, faucet, amount) = match asset {
                Asset::Fungible(fungible_asset) => {
                    let (faucet, amount) =
                        faucet_details_map.format_fungible_asset(&fungible_asset)?;
                    ("Fungible Asset", faucet, amount)
                },
                Asset::NonFungible(non_fungible_asset) => {
                    // TODO: Display non-fungible assets more clearly.
                    (
                        "Non Fungible Asset",
                        non_fungible_asset.faucet_id_prefix().to_hex(),
                        1.0.to_string(),
                    )
                },
            };
            table.add_row(vec![asset_type, &faucet, &amount.to_string()]);
        }

        println!("{table}\n");
    }

    // Storage Table
    {
        let account_storage = account.storage();

        println!("Storage: \n");

        let mut table =
            create_dynamic_table(&["Item Slot Index", "Item Slot Type", "Value/Commitment"]);

        for (idx, entry) in account_storage.slots().iter().enumerate() {
            let item = account_storage
                .get_item(u8::try_from(idx).expect("there are no more than 256 slots"))
                .map_err(|err| CliError::Account(err, "Index out of bounds".to_string()))?;

            // Last entry is reserved so I don't think the user cares about it. Also, to keep the
            // output smaller, if the [StorageSlot] is a value and it's 0 we assume it's not
            // initialized and skip it
            if matches!(entry, StorageSlot::Value { .. }) && item == [ZERO; 4].into() {
                continue;
            }

            let slot_type = match entry {
                StorageSlot::Value(..) => "Value",
                StorageSlot::Map(..) => "Map",
            };
            table.add_row(vec![&idx.to_string(), slot_type, &item.to_hex()]);
        }
        println!("{table}\n");
    }

    // Account code
    if with_code {
        println!("Code: \n");

        let mut table = create_dynamic_table(&["Code"]);
        table.add_row(vec![&account.code().to_pretty_string()]);
        println!("{table}");
    }

    Ok(())
}

// HELPERS
// ================================================================================================

/// Prints a summary table with account information.
fn print_summary_table(account: &Account, cli_config: &CliConfig) -> Result<(), CliError> {
    let mut table = create_dynamic_table(&["Account Information"]);
    table
        .load_preset(presets::UTF8_HORIZONTAL_ONLY)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth);

    table.add_row(vec![Cell::new("Account ID (hex)"), Cell::new(account.id().to_string())]);
    table.add_row(vec![
        Cell::new("Account ID (bech32)"),
        Cell::new(account.id().to_bech32(cli_config.network.to_network_id()?)),
    ]);
    table.add_row(vec![
        Cell::new("Account Commitment"),
        Cell::new(account.commitment().to_string()),
    ]);
    table.add_row(vec![Cell::new("Type"), Cell::new(account_type_display_name(&account.id())?)]);
    table.add_row(vec![
        Cell::new("Storage mode"),
        Cell::new(account.id().storage_mode().to_string()),
    ]);
    table.add_row(vec![
        Cell::new("Code Commitment"),
        Cell::new(account.code().commitment().to_string()),
    ]);
    table.add_row(vec![
        Cell::new("Vault Root"),
        Cell::new(account.vault().asset_tree().root().to_string()),
    ]);
    table.add_row(vec![
        Cell::new("Storage Root"),
        Cell::new(account.storage().commitment().to_string()),
    ]);
    table.add_row(vec![Cell::new("Nonce"), Cell::new(account.nonce().as_int().to_string())]);

    println!("{table}\n");
    Ok(())
}

/// Returns a display name for the account type.
fn account_type_display_name(account_id: &AccountId) -> Result<String, CliError> {
    Ok(match account_id.account_type() {
        AccountType::FungibleFaucet => {
            let faucet_details_map = load_faucet_details_map()?;
            let token_symbol = faucet_details_map.get_token_symbol_or_default(account_id);

            format!("Fungible faucet (token symbol: {token_symbol})")
        },
        AccountType::NonFungibleFaucet => "Non-fungible faucet".to_string(),
        AccountType::RegularAccountImmutableCode => "Regular".to_string(),
        AccountType::RegularAccountUpdatableCode => "Regular (updatable)".to_string(),
    })
}

/// Loads config file and displays current default account ID.
fn display_default_account_id() -> Result<(), CliError> {
    let (cli_config, _) = load_config_file()?;

    let default_account = cli_config.default_account_id.ok_or(CliError::Config(
        "Default account".to_string().into(),
        "No default account found in the configuration file".to_string(),
    ))?;
    println!("Current default account ID: {default_account}");
    Ok(())
}

/// Sets the provided account ID as the default account ID if provided. Unsets the current default
/// account ID if `None` is provided.
pub(crate) fn set_default_account(account_id: Option<AccountId>) -> Result<(), CliError> {
    // load config
    let (mut current_config, config_path) = load_config_file()?;

    // set default account
    current_config.default_account_id = account_id.map(AccountId::to_hex);

    update_config(&config_path, &current_config)
}

/// Sets the provided account ID as the default account and updates the config file, if not set
/// already.
pub(crate) fn maybe_set_default_account(
    current_config: &mut CliConfig,
    account_id: AccountId,
) -> Result<(), CliError> {
    if current_config.default_account_id.is_some() {
        return Ok(());
    }

    set_default_account(Some(account_id))?;
    let account_id = account_id.to_hex();
    println!("Setting account {account_id} as the default account ID.");
    println!("You can unset it with `{CLIENT_BINARY_NAME} account --default none`.");
    current_config.default_account_id = Some(account_id);

    Ok(())
}

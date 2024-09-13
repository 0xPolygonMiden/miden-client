use clap::Parser;
use miden_client::{
    accounts::{AccountId, AccountStorage, AccountType, StorageSlotType},
    assets::Asset,
    auth::TransactionAuthenticator,
    crypto::FeltRng,
    rpc::NodeRpcClient,
    store::Store,
    Client, ZERO,
};

use crate::{
    config::CliConfig,
    create_dynamic_table,
    utils::{load_config_file, load_faucet_details_map, parse_account_id, update_config},
    CLIENT_BINARY_NAME,
};

// ACCOUNT COMMAND
// ================================================================================================

#[derive(Default, Debug, Clone, Parser)]
/// View and manage accounts. Defaults to `list` command.
pub struct AccountCmd {
    /// List all accounts monitored by this client (default action)
    #[clap(short, long, group = "action")]
    list: bool,
    /// Show details of the account for the specified ID or hex prefix
    #[clap(short, long, group = "action", value_name = "ID")]
    show: Option<String>,
    /// Manages default account for transaction execution
    ///
    /// If no ID is provided it will display the current default account ID.
    /// If "none" is provided it will remove the default account else
    /// it will set the default account to the provided ID
    #[clap(short, long, group = "action", value_name = "ID")]
    default: Option<Option<String>>,
}

impl AccountCmd {
    pub fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        match self {
            AccountCmd {
                list: false,
                show: Some(id),
                default: None,
            } => {
                let account_id = parse_account_id(&client, id)?;
                show_account(client, account_id)?;
            },
            AccountCmd {
                list: false,
                show: None,
                default: Some(id),
            } => {
                match id {
                    None => {
                        display_default_account_id()?;
                    },
                    Some(id) => {
                        let default_account = if id == "none" {
                            None
                        } else {
                            let account_id: AccountId = AccountId::from_hex(id)
                                .map_err(|_| "Input number was not a valid Account Id")?;

                            // Check whether we're tracking that account
                            let (account, _) = client.get_account_header_by_id(account_id)?;

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
                list_accounts(client)?;
            },
        }
        Ok(())
    }
}

// LIST ACCOUNTS
// ================================================================================================

fn list_accounts<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: Client<N, R, S, A>,
) -> Result<(), String> {
    let accounts = client.get_account_headers()?;

    let mut table = create_dynamic_table(&["Account ID", "Type", "Storage Mode", "Nonce"]);
    for (acc, _acc_seed) in accounts.iter() {
        table.add_row(vec![
            acc.id().to_string(),
            account_type_display_name(&acc.id())?,
            acc.id().storage_mode().to_string(),
            acc.nonce().as_int().to_string(),
        ]);
    }

    println!("{table}");
    Ok(())
}

pub fn show_account<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    client: Client<N, R, S, A>,
    account_id: AccountId,
) -> Result<(), String> {
    let (account, _) = client.get_account(account_id)?;

    let mut table = create_dynamic_table(&[
        "Account ID",
        "Account Hash",
        "Type",
        "Storage mode",
        "Code Commitment",
        "Vault Root",
        "Storage Root",
        "Nonce",
    ]);
    table.add_row(vec![
        account.id().to_string(),
        account.hash().to_string(),
        account_type_display_name(&account_id)?,
        account_id.storage_mode().to_string(),
        account.code().commitment().to_string(),
        account.vault().asset_tree().root().to_string(),
        account.storage().commitment().to_string(),
        account.nonce().as_int().to_string(),
    ]);
    println!("{table}\n");

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
                    ("Non Fungible Asset", non_fungible_asset.faucet_id().to_hex(), 1.0.to_string())
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

        let mut table = create_dynamic_table(&[
            "Item Slot Index",
            "Item Slot Type",
            "Value Arity",
            "Value/Commitment",
        ]);
        for (idx, entry) in account_storage.slots().iter().enumerate() {
            // TODO: Re-add this code, and make sure it supports the recent storage changes

            // let item = account_storage.get_item(idx as u8);

            // // Last entry is reserved so I don't think the user cares about it. Also, to keep the
            // // output smaller, if the [StorageSlot] is a value and it's 0 we assume it's not
            // // initialized and skip it
            // if matches!(entry, StorageSlot::Value)
            //     && item == [ZERO; 4].into()
            // {
            //     continue;
            // }

            // let (slot_type, arity) = match entry {
            //     StorageSlotType::Value  => ("Value", arity),
            //     StorageSlotType::Array { depth: _depth, value_arity } => ("Array", value_arity),
            //     StorageSlotType::Map => ("Map", value_arity),
            // };
            // table.add_row(vec![&idx.to_string(), slot_type, &arity.to_string(),
            // &item.to_hex()?]);
        }
        println!("{table}\n");
    }

    Ok(())
}

// HELPERS
// ================================================================================================

fn account_type_display_name(account_id: &AccountId) -> Result<String, String> {
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

/// Loads config file and displays current default account ID
fn display_default_account_id() -> Result<(), String> {
    let (cli_config, _) = load_config_file()?;

    let default_account = cli_config.default_account_id.ok_or(
        "No default account found in the CLI options from the client config file.".to_string(),
    )?;
    println!("Current default account ID: {default_account}");
    Ok(())
}

/// Sets the provided account ID as the default account ID if provided. Unsets the current default
/// account ID if `None` is provided.
pub(crate) fn set_default_account(account_id: Option<AccountId>) -> Result<(), String> {
    // load config
    let (mut current_config, config_path) = load_config_file()?;

    // set default account
    current_config.default_account_id = account_id.map(|id| id.to_hex());

    update_config(&config_path, current_config)
}

/// Sets the provided account ID as the default account and updates the config file, if not set
/// already.
pub(crate) fn maybe_set_default_account(
    current_config: &mut CliConfig,
    account_id: AccountId,
) -> Result<(), String> {
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

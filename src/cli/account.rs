use clap::Parser;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
use miden_client::{rpc::NodeRpcClient, store::Store, Client};
use miden_objects::{
    accounts::{AccountId, AccountStorage, AccountType, AuthSecretKey, StorageSlotType},
    assets::Asset,
    crypto::{dsa::rpo_falcon512::SK_LEN, rand::FeltRng},
    ZERO,
};
use miden_tx::{
    auth::TransactionAuthenticator,
    utils::{bytes_to_hex_string, Serializable},
};

use super::utils::{load_config_file, parse_account_id, update_config};
use crate::cli::create_dynamic_table;

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
                            let (account, _) = client.get_account_stub_by_id(account_id)?;

                            Some(account.id().to_hex())
                        };

                        // load config
                        let (mut cli_config, config_path) = load_config_file()?;

                        // set default account
                        cli_config.default_account_id.clone_from(&default_account);

                        if let Some(id) = default_account {
                            println!("Setting default account to {id}...");
                        } else {
                            println!("Removing default account...");
                        }

                        update_config(&config_path, cli_config)?;
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
    let accounts = client.get_account_stubs()?;

    let mut table = create_dynamic_table(&["Account ID", "Type", "Storage Mode", "Nonce"]);
    accounts.iter().for_each(|(acc, _acc_seed)| {
        table.add_row(vec![
            acc.id().to_string(),
            account_type_display_name(&acc.id().account_type()),
            storage_type_display_name(&acc.id()),
            acc.nonce().as_int().to_string(),
        ]);
    });

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
        "Code Root",
        "Vault Root",
        "Storage Root",
        "Nonce",
    ]);
    table.add_row(vec![
        account.id().to_string(),
        account.hash().to_string(),
        account_type_display_name(&account.account_type()),
        storage_type_display_name(&account_id),
        account.code().root().to_string(),
        account.vault().asset_tree().root().to_string(),
        account.storage().root().to_string(),
        account.nonce().as_int().to_string(),
    ]);
    println!("{table}\n");

    // Vault Table
    {
        let assets = account.vault().assets();

        println!("Assets: ");

        let mut table = create_dynamic_table(&["Asset Type", "Faucet ID", "Amount"]);
        for asset in assets {
            let (asset_type, faucet_id, amount) = match asset {
                Asset::Fungible(fungible_asset) => {
                    ("Fungible Asset", fungible_asset.faucet_id(), fungible_asset.amount())
                },
                Asset::NonFungible(non_fungible_asset) => {
                    ("Non Fungible Asset", non_fungible_asset.faucet_id(), 1)
                },
            };
            table.add_row(vec![asset_type, &faucet_id.to_hex(), &amount.to_string()]);
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
        for (idx, entry) in account_storage.layout().iter().enumerate() {
            let item = account_storage.get_item(idx as u8);

            // Last entry is reserved so I don't think the user cares about it Also, to keep the
            // output smaller, if the [StorageSlotType] is a value and it's 0 we assume it's not
            // initialized and skip it
            if idx == AccountStorage::SLOT_LAYOUT_COMMITMENT_INDEX as usize {
                continue;
            }
            if matches!(entry, StorageSlotType::Value { value_arity: _value_arity })
                && item == [ZERO; 4].into()
            {
                continue;
            }

            let (slot_type, arity) = match entry {
                StorageSlotType::Value { value_arity } => ("Value", value_arity),
                StorageSlotType::Array { depth: _depth, value_arity } => ("Array", value_arity),
                StorageSlotType::Map { value_arity } => ("Map", value_arity),
            };
            table.add_row(vec![&idx.to_string(), slot_type, &arity.to_string(), &item.to_hex()]);
        }
        println!("{table}\n");
    }

    // Keys table
    {
        let auth_info = client.get_account_auth(account_id)?;

        match auth_info {
            AuthSecretKey::RpoFalcon512(key_pair) => {
                let auth_info: [u8; SK_LEN] = key_pair
                    .to_bytes()
                    .try_into()
                    .expect("Array size is const and should always exactly fit SecretKey");

                let mut table = Table::new();
                table
                    .load_preset(presets::UTF8_HORIZONTAL_ONLY)
                    .set_content_arrangement(ContentArrangement::DynamicFullWidth)
                    .set_header(vec![Cell::new("Key Pair").add_attribute(Attribute::Bold)]);

                table.add_row(vec![format!("0x{}\n", bytes_to_hex_string(auth_info))]);
                println!("{table}\n");
            },
        };
    }

    // Code related table
    {
        let module = account.code().module();
        let procedure_digests = account.code().procedures();

        println!("Account Code Info:");

        let mut table = create_dynamic_table(&["Procedure Digests"]);

        for digest in procedure_digests {
            table.add_row(vec![digest.to_hex()]);
        }
        println!("{table}\n");

        let mut code_table = create_dynamic_table(&["Code"]);
        code_table.load_preset(presets::UTF8_HORIZONTAL_ONLY);
        code_table.add_row(vec![&module]);
        println!("{code_table}\n");
    }

    Ok(())
}

// HELPERS
// ================================================================================================

fn account_type_display_name(account_type: &AccountType) -> String {
    match account_type {
        AccountType::FungibleFaucet => "Fungible faucet",
        AccountType::NonFungibleFaucet => "Non-fungible faucet",
        AccountType::RegularAccountImmutableCode => "Regular",
        AccountType::RegularAccountUpdatableCode => "Regular (updatable)",
    }
    .to_string()
}

fn storage_type_display_name(account: &AccountId) -> String {
    match account.is_on_chain() {
        true => "On-chain",
        false => "Off-chain",
    }
    .to_string()
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

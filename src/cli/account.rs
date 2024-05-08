use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
use miden_client::{
    client::{accounts, rpc::NodeRpcClient, Client},
    config::{CliConfig, ClientConfig},
    store::Store,
};
use miden_objects::{
    accounts::{AccountId, AccountStorage, AccountType, StorageSlotType},
    assets::{Asset, TokenSymbol},
    crypto::{dsa::rpo_falcon512::SK_LEN, rand::FeltRng},
    ZERO,
};
use miden_tx::utils::{bytes_to_hex_string, Serializable};

use super::{load_config, parse_account_id, update_config, CLIENT_CONFIG_FILE_NAME};
use crate::cli::create_dynamic_table;

// ACCOUNT COMMAND
// ================================================================================================

#[derive(Default, Debug, Clone, Parser)]
pub enum AccountCmd {
    /// List all accounts monitored by this client
    #[default]
    #[clap(short_flag = 'l', long_flag = "list")]
    List,
    /// Show details of the account for the specified ID or hex prefix
    #[clap(short_flag = 's', long_flag = "show")]
    Show { id: String },
    /// Create new account and store it locally
    #[clap(short_flag = 'n', long_flag = "new")]
    New {
        #[clap(subcommand)]
        template: AccountTemplate,
    },
    /// Set/Unset default accounts for transaction execution
    #[clap(short_flag = 'd', long_flag = "default")]
    Default {
        #[clap(subcommand)]
        default_cmd: DefaultAccountCmd,
    },
}

#[derive(Debug, Parser, Clone)]
#[clap()]
pub enum DefaultAccountCmd {
    /// Turn an account into the default sender account
    Set {
        #[clap()]
        id: String,
    },
    /// Show current default account
    Show,
    /// Clear the default account setting
    Unset,
}

#[derive(Debug, Parser, Clone)]
#[clap()]
pub enum AccountTemplate {
    /// Creates a basic account (Regular account with immutable code)
    BasicImmutable {
        #[clap(short, long, value_enum, default_value_t = AccountStorageMode::OffChain)]
        storage_type: AccountStorageMode,
    },
    /// Creates a basic account (Regular account with mutable code)
    BasicMutable {
        #[clap(short, long, value_enum, default_value_t = AccountStorageMode::OffChain)]
        storage_type: AccountStorageMode,
    },
    /// Creates a faucet for fungible tokens
    FungibleFaucet {
        #[clap(short, long)]
        token_symbol: String,
        #[clap(short, long)]
        decimals: u8,
        #[clap(short, long)]
        max_supply: u64,
        #[clap(short, long, value_enum, default_value_t = AccountStorageMode::OffChain)]
        storage_type: AccountStorageMode,
    },
    /// Creates a faucet for non-fungible tokens
    NonFungibleFaucet {
        #[clap(short, long, value_enum, default_value_t = AccountStorageMode::OffChain)]
        storage_type: AccountStorageMode,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AccountStorageMode {
    OffChain,
    OnChain,
}

impl From<AccountStorageMode> for accounts::AccountStorageMode {
    fn from(value: AccountStorageMode) -> Self {
        match value {
            AccountStorageMode::OffChain => accounts::AccountStorageMode::Local,
            AccountStorageMode::OnChain => accounts::AccountStorageMode::OnChain,
        }
    }
}

impl From<&AccountStorageMode> for accounts::AccountStorageMode {
    fn from(value: &AccountStorageMode) -> Self {
        accounts::AccountStorageMode::from(*value)
    }
}

impl AccountCmd {
    pub fn execute<N: NodeRpcClient, R: FeltRng, S: Store>(
        &self,
        mut client: Client<N, R, S>,
    ) -> Result<(), String> {
        match self {
            AccountCmd::List => {
                list_accounts(client)?;
            },
            AccountCmd::New { template } => {
                let client_template = match template {
                    AccountTemplate::BasicImmutable { storage_type: storage_mode } => {
                        accounts::AccountTemplate::BasicWallet {
                            mutable_code: false,
                            storage_mode: storage_mode.into(),
                        }
                    },
                    AccountTemplate::BasicMutable { storage_type: storage_mode } => {
                        accounts::AccountTemplate::BasicWallet {
                            mutable_code: true,
                            storage_mode: storage_mode.into(),
                        }
                    },
                    AccountTemplate::FungibleFaucet {
                        token_symbol,
                        decimals,
                        max_supply,
                        storage_type: storage_mode,
                    } => accounts::AccountTemplate::FungibleFaucet {
                        token_symbol: TokenSymbol::new(token_symbol)
                            .map_err(|err| format!("error: token symbol is invalid: {}", err))?,
                        decimals: *decimals,
                        max_supply: *max_supply,
                        storage_mode: storage_mode.into(),
                    },
                    AccountTemplate::NonFungibleFaucet { storage_type: _ } => todo!(),
                };
                let (_new_account, _account_seed) = client.new_account(client_template)?;
            },
            AccountCmd::Show { id } => {
                let account_id = parse_account_id(&client, id)?;
                show_account(client, account_id)?;
            },
            AccountCmd::Default {
                default_cmd: DefaultAccountCmd::Set { id },
            } => {
                let account_id: AccountId = AccountId::from_hex(id)
                    .map_err(|_| "Input number was not a valid Account Id")?;

                // Check whether we're tracking that account
                let (account, _) = client.get_account_stub_by_id(account_id)?;

                // load config
                let (mut current_config, config_path) = load_config_file()?;

                // set default account
                current_config.cli = Some(CliConfig {
                    default_account_id: Some(account.id().to_hex()),
                });

                update_config(&config_path, current_config)?;
            },
            AccountCmd::Default { default_cmd: DefaultAccountCmd::Unset } => {
                let (mut current_config, path) = load_config_file()?;

                // unset default account
                current_config.cli.replace(CliConfig { default_account_id: None });

                update_config(&path, current_config)?;
            },
            AccountCmd::Default { default_cmd: DefaultAccountCmd::Show } => {
                display_default_account_id()?;
            },
        }
        Ok(())
    }
}

// LIST ACCOUNTS
// ================================================================================================

fn list_accounts<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: Client<N, R, S>,
) -> Result<(), String> {
    let accounts = client.get_account_stubs()?;

    let mut table = create_dynamic_table(&[
        "Account ID",
        "Code Root",
        "Vault Root",
        "Storage Root",
        "Type",
        "Storage mode",
        "Nonce",
    ]);
    accounts.iter().for_each(|(acc, _acc_seed)| {
        table.add_row(vec![
            acc.id().to_string(),
            acc.code_root().to_string(),
            acc.vault_root().to_string(),
            acc.storage_root().to_string(),
            account_type_display_name(&acc.id().account_type()),
            storage_type_display_name(&acc.id()),
            acc.nonce().as_int().to_string(),
        ]);
    });

    println!("{table}");
    Ok(())
}

pub fn show_account<N: NodeRpcClient, R: FeltRng, S: Store>(
    client: Client<N, R, S>,
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
            miden_client::store::AuthInfo::RpoFalcon512(key_pair) => {
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
    let (miden_client_config, _) = load_config_file()?;
    let cli_config = miden_client_config
        .cli
        .ok_or("No CLI options found in the client config file".to_string())?;

    let default_account = cli_config.default_account_id.ok_or(
        "No default account found in the CLI options from the client config file.".to_string(),
    )?;
    println!("Current default account ID: {default_account}");
    Ok(())
}

/// Loads config file from current directory and default filename and returns it alongside its path
fn load_config_file() -> Result<(ClientConfig, PathBuf), String> {
    let mut current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
    current_dir.push(CLIENT_CONFIG_FILE_NAME);
    let config_path = current_dir.as_path();

    let client_config = load_config(config_path)?;

    Ok((client_config, config_path.into()))
}

use std::{fs, path::PathBuf};

use clap::Parser;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
use miden_client::{
    client::{accounts, rpc::NodeRpcClient, Client},
    store::Store,
};
use miden_objects::{
    accounts::{AccountData, AccountId, AccountStorage, AccountType, StorageSlotType},
    assets::{Asset, TokenSymbol},
    crypto::dsa::rpo_falcon512::KeyPair,
    ZERO,
};
use miden_tx::utils::{bytes_to_hex_string, Deserializable, Serializable};
use tracing::info;

use crate::cli::create_dynamic_table;

// ACCOUNT COMMAND
// ================================================================================================

#[derive(Debug, Clone, Parser)]
#[clap(about = "Create accounts and inspect account details")]
pub enum AccountCmd {
    /// List all accounts monitored by this client
    #[clap(short_flag = 'l')]
    List,

    /// Show details of the account for the specified ID
    #[clap(short_flag = 's')]
    Show {
        // TODO: We should create a value parser for catching input parsing errors earlier (ie AccountID) once complexity grows
        #[clap()]
        id: Option<String>,
        #[clap(short, long, default_value_t = false)]
        keys: bool,
        #[clap(short, long, default_value_t = false)]
        vault: bool,
        #[clap(short, long, default_value_t = false)]
        storage: bool,
        #[clap(short, long, default_value_t = false)]
        code: bool,
    },
    /// Create new account and store it locally
    #[clap(short_flag = 'n')]
    New {
        #[clap(subcommand)]
        template: AccountTemplate,
    },
    /// Import accounts from binary files (with .mac extension)
    #[clap(short_flag = 'i')]
    Import {
        /// Paths to the files that contains the account data
        #[arg()]
        filenames: Vec<PathBuf>,
    },
}

#[derive(Debug, Parser, Clone)]
#[clap()]
pub enum AccountTemplate {
    /// Creates a basic account (Regular account with immutable code)
    BasicImmutable,
    /// Creates a basic account (Regular account with mutable code)
    BasicMutable,
    /// Creates a faucet for fungible tokens
    FungibleFaucet {
        #[clap(short, long)]
        token_symbol: String,
        #[clap(short, long)]
        decimals: u8,
        #[clap(short, long)]
        max_supply: u64,
    },
    /// Creates a faucet for non-fungible tokens
    NonFungibleFaucet,
}

impl AccountCmd {
    pub fn execute<N: NodeRpcClient, S: Store>(
        &self,
        mut client: Client<N, S>,
    ) -> Result<(), String> {
        match self {
            AccountCmd::List => {
                list_accounts(client)?;
            },
            AccountCmd::New { template } => {
                let client_template = match template {
                    AccountTemplate::BasicImmutable => accounts::AccountTemplate::BasicWallet {
                        mutable_code: false,
                        storage_mode: accounts::AccountStorageMode::Local,
                    },
                    AccountTemplate::BasicMutable => accounts::AccountTemplate::BasicWallet {
                        mutable_code: true,
                        storage_mode: accounts::AccountStorageMode::Local,
                    },
                    AccountTemplate::FungibleFaucet {
                        token_symbol,
                        decimals,
                        max_supply,
                    } => accounts::AccountTemplate::FungibleFaucet {
                        token_symbol: TokenSymbol::new(token_symbol)
                            .map_err(|err| format!("error: token symbol is invalid: {}", err))?,
                        decimals: *decimals,
                        max_supply: *max_supply,
                        storage_mode: accounts::AccountStorageMode::Local,
                    },
                    AccountTemplate::NonFungibleFaucet => todo!(),
                };
                let (_new_account, _account_seed) = client.new_account(client_template)?;
            },
            AccountCmd::Show { id: None, .. } => {
                todo!("Default accounts are not supported yet")
            },
            AccountCmd::Show {
                id: Some(v),
                keys,
                vault,
                storage,
                code,
            } => {
                let account_id: AccountId = AccountId::from_hex(v)
                    .map_err(|_| "Input number was not a valid Account Id")?;
                show_account(client, account_id, *keys, *vault, *storage, *code)?;
            },
            AccountCmd::Import { filenames } => {
                validate_paths(filenames, "mac")?;
                for filename in filenames {
                    import_account(&mut client, filename)?;
                }
                println!("Imported {} accounts.", filenames.len());
            },
        }
        Ok(())
    }
}

// LIST ACCOUNTS
// ================================================================================================

fn list_accounts<N: NodeRpcClient, S: Store>(client: Client<N, S>) -> Result<(), String> {
    let accounts = client.get_accounts()?;

    let mut table = create_dynamic_table(&[
        "Account ID",
        "Code Root",
        "Vault Root",
        "Storage Root",
        "Type",
        "Nonce",
    ]);
    accounts.iter().for_each(|(acc, _acc_seed)| {
        table.add_row(vec![
            acc.id().to_string(),
            acc.code_root().to_string(),
            acc.vault_root().to_string(),
            acc.storage_root().to_string(),
            account_type_display_name(&acc.id().account_type()),
            acc.nonce().as_int().to_string(),
        ]);
    });

    println!("{table}");
    Ok(())
}

pub fn show_account<N: NodeRpcClient, S: Store>(
    client: Client<N, S>,
    account_id: AccountId,
    show_keys: bool,
    show_vault: bool,
    show_storage: bool,
    show_code: bool,
) -> Result<(), String> {
    let (account, _account_seed) = client.get_account(account_id)?;
    let mut table = create_dynamic_table(&[
        "Account ID",
        "Account Hash",
        "Type",
        "Code Root",
        "Vault Root",
        "Storage Root",
        "Nonce",
    ]);
    table.add_row(vec![
        account.id().to_string(),
        account.hash().to_string(),
        account_type_display_name(&account.account_type()),
        account.code().root().to_string(),
        account.vault().asset_tree().root().to_string(),
        account.storage().root().to_string(),
        account.nonce().as_int().to_string(),
    ]);
    println!("{table}\n");

    if show_vault {
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

    if show_storage {
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
            if matches!(
                entry,
                StorageSlotType::Value {
                    value_arity: _value_arity
                }
            ) && item == [ZERO; 4].into()
            {
                continue;
            }

            let (slot_type, arity) = match entry {
                StorageSlotType::Value { value_arity } => ("Value", value_arity),
                StorageSlotType::Array {
                    depth: _depth,
                    value_arity,
                } => ("Array", value_arity),
                StorageSlotType::Map { value_arity } => ("Map", value_arity),
            };
            table.add_row(vec![&idx.to_string(), slot_type, &arity.to_string(), &item.to_hex()]);
        }
        println!("{table}\n");
    }

    if show_keys {
        let auth_info = client.get_account_auth(account_id)?;

        match auth_info {
            miden_client::store::AuthInfo::RpoFalcon512(key_pair) => {
                const KEY_PAIR_SIZE: usize = std::mem::size_of::<KeyPair>();
                let auth_info: [u8; KEY_PAIR_SIZE] = key_pair
                    .to_bytes()
                    .try_into()
                    .expect("Array size is const and should always exactly fit KeyPair");

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

    if show_code {
        let module = account.code().module();
        let procedure_digests = account.code().procedures();

        println!("Account Code Info:");

        let mut table = create_dynamic_table(&["Procedure Digests"]);
        for digest in procedure_digests {
            table.add_row(vec![digest.to_hex()]);
        }
        println!("{table}\n");

        let mut code_table = create_dynamic_table(&["Code"]);
        code_table.add_row(vec![&module]);
        println!("{code_table}\n");
    }

    Ok(())
}

// IMPORT ACCOUNT
// ================================================================================================

fn import_account<N: NodeRpcClient, S: Store>(
    client: &mut Client<N, S>,
    filename: &PathBuf,
) -> Result<(), String> {
    info!(
        "Attempting to import account data from {}...",
        fs::canonicalize(filename).map_err(|err| err.to_string())?.as_path().display()
    );
    let account_data_file_contents = fs::read(filename).map_err(|err| err.to_string())?;
    let account_data =
        AccountData::read_from_bytes(&account_data_file_contents).map_err(|err| err.to_string())?;
    let account_id = account_data.account.id();

    client.import_account(account_data)?;
    println!("Imported account with ID: {}", account_id);

    Ok(())
}

// HELPERS
// ================================================================================================

/// Checks that all files exist, otherwise returns an error. It also ensures that all files have a
/// specific extension
fn validate_paths(
    paths: &[PathBuf],
    expected_extension: &str,
) -> Result<(), String> {
    let invalid_path = paths.iter().find(|path| {
        !path.exists() || path.extension().map_or(false, |ext| ext != expected_extension)
    });

    if let Some(path) = invalid_path {
        Err(format!(
            "The path `{}` does not exist or does not have the appropiate extension",
            path.to_string_lossy()
        )
        .to_string())
    } else {
        Ok(())
    }
}

fn account_type_display_name(account_type: &AccountType) -> String {
    match account_type {
        AccountType::FungibleFaucet => "Fungible faucet",
        AccountType::NonFungibleFaucet => "Non-fungible faucet",
        AccountType::RegularAccountImmutableCode => "Regular",
        AccountType::RegularAccountUpdatableCode => "Regular (updatable)",
    }
    .to_string()
}

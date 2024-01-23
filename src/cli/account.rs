use std::{fs, path::PathBuf};

use clap::Parser;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
use crypto::{
    dsa::rpo_falcon512::KeyPair,
    utils::{bytes_to_hex_string, Deserializable, Serializable},
};
use miden_client::client::accounts;

use super::Client;
use objects::{
    accounts::{AccountData, AccountId},
    assets::TokenSymbol,
    Digest,
};

// ACCOUNT COMMAND
// ================================================================================================

#[derive(Debug, Clone, Parser)]
#[clap(about = "View accounts and account details")]
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
        /// Path to the file that contains the input note data
        #[clap(short, long, num_args = 1..)]
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
    pub fn execute(&self, mut client: Client) -> Result<(), String> {
        match self {
            AccountCmd::List => {
                list_accounts(client)?;
            }
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
                let (_new_account, _account_seed) = client
                    .new_account(client_template)
                    .map_err(|err| err.to_string())?;
            }
            AccountCmd::Show { id: None, .. } => {
                todo!("Default accounts are not supported yet")
            }
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
            }
            AccountCmd::Import { filenames } => {
                validate_paths(filenames, None)?;
                for filename in filenames {
                    import_account(&mut client, filename)?;
                }
            }
        }
        Ok(())
    }
}

// LIST ACCOUNTS
// ================================================================================================

fn list_accounts(client: Client) -> Result<(), String> {
    let accounts = client.get_accounts().map_err(|err| err.to_string())?;

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec![
            Cell::new("account id").add_attribute(Attribute::Bold),
            Cell::new("code root").add_attribute(Attribute::Bold),
            Cell::new("vault root").add_attribute(Attribute::Bold),
            Cell::new("storage root").add_attribute(Attribute::Bold),
            Cell::new("nonce").add_attribute(Attribute::Bold),
        ]);

    accounts.iter().for_each(|(acc, _acc_seed)| {
        table.add_row(vec![
            acc.id().to_string(),
            acc.code_root().to_string(),
            acc.vault_root().to_string(),
            acc.storage_root().to_string(),
            acc.nonce().to_string(),
        ]);
    });

    println!("{table}");
    Ok(())
}

pub fn show_account(
    client: Client,
    account_id: AccountId,
    show_keys: bool,
    show_vault: bool,
    show_storage: bool,
    show_code: bool,
) -> Result<(), String> {
    let (account, account_seed) = client
        .get_account_stub_by_id(account_id)
        .map_err(|err| err.to_string())?;

    let formatted_seed = Digest::from(account_seed).to_string();

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(vec![
            Cell::new("account id").add_attribute(Attribute::Bold),
            Cell::new("code root").add_attribute(Attribute::Bold),
            Cell::new("vault root").add_attribute(Attribute::Bold),
            Cell::new("storage root").add_attribute(Attribute::Bold),
            Cell::new("nonce").add_attribute(Attribute::Bold),
            Cell::new("account seed").add_attribute(Attribute::Bold),
        ]);

    table.add_row(vec![
        account.id().to_string(),
        account.code_root().to_string(),
        account.vault_root().to_string(),
        account.storage_root().to_string(),
        account.nonce().to_string(),
        formatted_seed,
    ]);

    println!("{table}\n");

    if show_keys {
        let auth_info = client
            .get_account_auth(account_id)
            .map_err(|err| err.to_string())?;

        match auth_info {
            miden_client::store::accounts::AuthInfo::RpoFalcon512(key_pair) => {
                const KEY_PAIR_SIZE: usize = std::mem::size_of::<KeyPair>();
                let auth_info: [u8; KEY_PAIR_SIZE] = key_pair
                    .to_bytes()
                    .try_into()
                    .expect("Array size is const and should always exactly fit KeyPair");
                println!("Key pair:\n0x{}", bytes_to_hex_string(auth_info));
            }
        };
    }

    if show_vault {
        let assets = client
            .get_vault_assets(account.vault_root())
            .map_err(|err| err.to_string())?;

        println!(
            "Vault assets: {}\n",
            serde_json::to_string(&assets).map_err(|_| "Error serializing account assets")?
        );
    }

    if show_storage {
        let account_storage = client
            .get_account_storage(account.storage_root())
            .map_err(|err| err.to_string())?;

        println!(
            "Storage: {}\n",
            serde_json::to_string(&account_storage.slots())
                .map_err(|_| "Error serializing account storage")?
        );
    }

    if show_code {
        let (procedure_digests, module) = client
            .get_account_code(account.code_root())
            .map_err(|err| err.to_string())?;

        println!(
            "Procedure digests:\n{}\n",
            serde_json::to_string(&procedure_digests)
                .map_err(|_| "Error serializing account storage for display")?
        );
        println!("Module AST:\n{}\n", module);
    }

    Ok(())
}

// IMPORT INPUT NOTE
// ================================================================================================
fn import_account(client: &mut Client, filename: &PathBuf) -> Result<(), String> {
    let account_data_file_contents = fs::read(filename).map_err(|err| err.to_string())?;
    let account_data =
        AccountData::read_from_bytes(&account_data_file_contents).map_err(|err| err.to_string())?;

    client.import_account(account_data)?;

    Ok(())
}

// HELPERS
// ================================================================================================

/// Checks that all files exist, otherwise returns an error. It can also validate that all files
/// have a specific extension
fn validate_paths(paths: &[PathBuf], expected_extension: Option<&str>) -> Result<(), String> {
    let invalid_path = if let Some(extension) = expected_extension {
        paths
            .iter()
            .find(|path| !path.exists() || path.extension().map_or(false, |ext| ext != extension))
    } else {
        paths.iter().find(|path| !path.exists())
    };

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

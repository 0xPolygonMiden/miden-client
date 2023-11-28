use clap::Parser;
use crypto::{dsa::rpo_falcon512::KeyPair, utils::Serializable, Felt};
use miden_client::{Client, ClientConfig};
use miden_lib::{faucets, AuthScheme};
use objects::{
    accounts::{AccountId, AccountType},
    assembly::AstSerdeOptions,
    assets::TokenSymbol,
};
use rand::Rng;

// ACCOUNT COMMAND
// ================================================================================================

#[derive(Debug, Clone, Parser)]
#[clap(about = "View accounts and account details")]
pub enum AccountCmd {
    /// List all accounts monitored by this client
    #[clap(short_flag = 'l')]
    List,

    /// Show details of the account for the specified ID
    #[clap(short_flag = 'v')]
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
        template: Option<AccountTemplate>,

        /// Executes a transaction that records the account on-chain
        #[clap(short, long, default_value_t = false)]
        deploy: bool,
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
    pub fn execute(&self) -> Result<(), String> {
        match self {
            AccountCmd::List => {
                list_accounts()?;
            }
            AccountCmd::New { template, deploy } => {
                new_account(template, *deploy)?;
            }
            AccountCmd::Show { id: None, .. } => {
                todo!("Setting default accounts is not supported yet")
            }
            AccountCmd::Show {
                id: Some(v),
                keys,
                vault,
                storage,
                code,
            } => {
                let clean_hex = v.to_lowercase();
                let clean_hex = clean_hex.strip_prefix("0x").unwrap_or(&clean_hex);

                // TODO: Improve errors
                let account_id = u64::from_str_radix(clean_hex, 16)
                    .map_err(|_| "Error parsing input Account Id as a hexadecimal number")?;
                let account_id: AccountId = account_id
                    .try_into()
                    .map_err(|_| "Input number was not a valid Account Id")?;

                show_account(account_id, *keys, *vault, *storage, *code)?;
            }
        }
        Ok(())
    }
}

// LIST ACCOUNTS
// ================================================================================================

fn list_accounts() -> Result<(), String> {
    println!("{}", "-".repeat(240));
    println!(
        "{0: <18} | {1: <66} | {2: <66} | {3: <66} | {4: <15}",
        "account id", "code root", "vault root", "storage root", "nonce",
    );
    println!("{}", "-".repeat(240));

    let client = Client::new(ClientConfig::default()).map_err(|err| err.to_string())?;
    let accounts = client.get_accounts().map_err(|err| err.to_string())?;

    for acct in accounts {
        println!(
            "{0: <18} | {1: <66} | {2: <66} | {3: <66} | {4: <15}",
            acct.id(),
            acct.code_root(),
            acct.vault_root(),
            acct.storage_root(),
            acct.nonce(),
        );
    }
    println!("{}", "-".repeat(240));
    Ok(())
}

// ACCOUNT NEW
// ================================================================================================

fn new_account(template: &Option<AccountTemplate>, deploy: bool) -> Result<(), String> {
    let client = Client::new(ClientConfig::default()).map_err(|err| err.to_string())?;

    if deploy {
        todo!("Recording the account on chain is not supported yet");
    }

    let key_pair: KeyPair =
        KeyPair::new().map_err(|err| format!("Error generating KeyPair: {}", err))?;
    let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 {
        pub_key: key_pair.public_key(),
    };

    let mut rng = rand::thread_rng();
    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = rng.gen();

    // TODO: as the client takes form, make errors more structured
    let (account, _) = match template {
        None => todo!("Generic account creation is not supported yet"),
        Some(AccountTemplate::BasicImmutable) => miden_lib::wallets::create_basic_wallet(
            init_seed,
            auth_scheme,
            AccountType::RegularAccountImmutableCode,
        ),
        Some(AccountTemplate::FungibleFaucet {
            token_symbol,
            decimals,
            max_supply,
        }) => {
            let max_supply = max_supply.to_le_bytes();
            faucets::create_basic_fungible_faucet(
                init_seed,
                TokenSymbol::new(token_symbol)
                    .expect("Hardcoded test token symbol creation should not panic"),
                *decimals,
                Felt::try_from(max_supply.as_slice())
                    .map_err(|_| "Maximum supply must fit into a field element")?,
                auth_scheme,
            )
        }
        Some(AccountTemplate::BasicMutable) => miden_lib::wallets::create_basic_wallet(
            init_seed,
            auth_scheme,
            AccountType::RegularAccountUpdatableCode,
        ),
        _ => todo!("Template not supported yet"),
    }
    .map_err(|err| err.to_string())?;

    // TODO: Make these inserts atomic through a single transaction
    client
        .store()
        .insert_account_code(account.code())
        .and_then(|_| client.store().insert_account_storage(account.storage()))
        .and_then(|_| client.store().insert_account_vault(account.vault()))
        .and_then(|_| client.store().insert_account(&account))
        .and_then(|_| client.store().insert_account_keys(account.id(), &key_pair))
        .map(|_| {
            println!(
                "Succesfully created and stored Account ID: {}",
                account.id()
            )
        })
        .map_err(|x| x.to_string())?;

    Ok(())
}

pub fn show_account(
    account_id: AccountId,
    keys: bool,
    vault: bool,
    storage: bool,
    code: bool,
) -> Result<(), String> {
    println!("{}", "-".repeat(240));
    println!(
        "{0: <18} | {1: <66} | {2: <66} | {3: <66} | {4: <15}",
        "account id", "code root", "vault root", "storage root", "nonce",
    );
    println!("{}", "-".repeat(240));

    let client = Client::new(ClientConfig::default()).map_err(|err| err.to_string())?;
    let account = client
        .get_account_by_id(account_id)
        .map_err(|err| err.to_string())?;

    println!(
        "{0: <18} | {1: <66} | {2: <66} | {3: <66} | {4: <15}",
        account.id(),
        account.code_root(),
        account.vault_root(),
        account.storage_root(),
        account.nonce(),
    );
    println!("{}\n", "-".repeat(240));

    if keys {
        let key_pair = client
            .get_account_keys(account_id)
            .map_err(|err| err.to_string())?;

        println!("Key pair: {}\n", hex::encode(key_pair.to_bytes()));
    }

    if vault {
        let assets = client
            .get_vault_assets(account.vault_root())
            .map_err(|err| err.to_string())?;

        println!(
            "Vault assets: {}\n",
            serde_json::to_string(&assets).map_err(|_| "Error serializing account assets")?
        );
    }

    if storage {
        let account_storage = client
            .get_account_storage(account.storage_root())
            .map_err(|err| err.to_string())?;

        println!(
            "Storage: {}\n",
            serde_json::to_string(&account_storage)
                .map_err(|_| "Error serializing account storage")?
        );
    }

    if code {
        let (procedure_digests, module) = client
            .get_account_code(account.code_root())
            .map_err(|err| err.to_string())?;

        println!(
            "Procedure digests: {}\n",
            serde_json::to_string(&procedure_digests)
                .map_err(|_| "Error serializing account storage for display")?
        );
        println!(
            "Module AST: {}\n",
            hex::encode(module.to_bytes(AstSerdeOptions {
                serialize_imports: true
            }))
        );
    }

    Ok(())
}

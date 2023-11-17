use clap::Parser;
use crypto::{dsa::rpo_falcon512::KeyPair, Felt};
use miden_client::{Client, ClientConfig};
use miden_lib::{faucets, AuthScheme};
use objects::{accounts::AccountType, assets::TokenSymbol};
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
        // TODO: We should create a value parser for stricter typing (ie AccountID) once complexity grows
        #[clap()]
        id: Option<String>,
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
            AccountCmd::Show { id: None } => todo!(),
            AccountCmd::Show { id: Some(v) } => {
                let clean_hex = v.to_lowercase();
                let clean_hex = clean_hex.strip_prefix("0x").unwrap_or(&clean_hex);

                // TODO: Improve errors
                let account_id = u64::from_str_radix(clean_hex, 16)
                    .map_err(|_| "Error parsing input Account Id as a hexadecimal number")?;
                let account_id: AccountId = account_id
                    .try_into()
                    .map_err(|_| "Input number was not a valid Account Id")?;

                show_account(account_id)?;
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
        .map(|_| {
            println!(
                "Succesfully created and stored Account ID: {}",
                account.id()
            )
        })
        .map_err(|x| x.to_string())?;

    Ok(())
}

pub fn create_basic_wallet(
    key_pair: KeyPair,
    init_seed: [u8; 32],
    account_type: AccountType,
) -> Result<(Account, Word), AccountError> {
    let account_code_string: String = "
    use.miden::wallets::basic->basic_wallet
    use.miden::eoa::basic

    export.basic_wallet::receive_asset
    export.basic_wallet::send_asset
    export.basic::auth_tx_rpo_falcon512

    "
    .to_string();
    let account_code_src: &str = &account_code_string;

    let account_code_ast =
        ModuleAst::parse(account_code_src).expect("Hardcoded program parsing should not panic");
    let account_assembler = miden_lib::assembler::assembler();
    let account_code = AccountCode::new(account_code_ast.clone(), &account_assembler)?;

    let account_storage =
        AccountStorage::new(vec![(0, key_pair.public_key().into())], MerkleStore::new())?;
    let account_vault = AccountVault::new(&[]).expect("Creating empty vault should not fail");

    let account_seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        false,
        account_code.root(),
        account_storage.root(),
    )?;
    let account_id = AccountId::new(account_seed, account_code.root(), account_storage.root())?;
    Ok((
        Account::new(
            account_id,
            account_vault,
            account_storage,
            account_code,
            ZERO,
        ),
        account_seed,
    ))
}

pub fn show_account(account_id: AccountId) -> Result<(), String> {
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

    println!("{}", "-".repeat(240));

    Ok(())
}

use clap::Parser;
use crypto::{dsa::rpo_falcon512::KeyPair, Felt};
use miden_client::{Client, ClientConfig};
use miden_lib::{faucets, AuthScheme};
use objects::{accounts::AccountType, assets::TokenSymbol};
use rand::Rng;
use tabled::{builder::Builder, settings::Style};

// ACCOUNT COMMAND
// ================================================================================================

#[derive(Debug, Clone, Parser)]
#[clap(about = "View accounts and account details")]
pub enum AccountCmd {
    /// List all accounts monitored by this client
    #[clap(short_flag = 'l')]
    List,

    /// View details of the account for the specified ID
    #[clap(short_flag = 'v')]
    View {
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
            AccountCmd::View { id: _ } => todo!(),
        }
        Ok(())
    }
}

// LIST ACCOUNTS
// ================================================================================================

fn list_accounts() -> Result<(), String> {
    let client = Client::new(ClientConfig::default()).map_err(|err| err.to_string())?;
    let accounts = client.get_accounts().map_err(|err| err.to_string())?;

    let mut builder = Builder::new();
    builder.push_record([
        "account id",
        "code root",
        "vault root",
        "storage root",
        "nonce",
    ]);

    for acct in accounts {
        builder.push_record([
            &acct.id().to_string(),
            &acct.code_root().to_string(),
            &acct.vault_root().to_string(),
            &acct.storage_root().to_string(),
            &acct.nonce().to_string(),
        ]);
    }

    let table = builder.build().with(Style::modern()).to_string();
    println!("{}", table);
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
        .insert_account(&account)
        .and_then(|_| client.store().insert_account_code(account.code()))
        .and_then(|_| client.store().insert_account_storage(account.storage()))
        .and_then(|_| client.store().insert_account_vault(account.vault()))
        .map(|_| {
            println!(
                "Succesfully created and stored Account ID: {}",
                account.id()
            )
        })
        .map_err(|x| x.to_string())?;

    Ok(())
}

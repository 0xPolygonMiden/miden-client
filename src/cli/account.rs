use clap::{Parser, ValueEnum};
use crypto::{dsa::rpo_falcon512::KeyPair, Felt};
use miden_client::{Client, ClientConfig};
use miden_lib::{faucets, wallets, AuthScheme};
use objects::assets::TokenSymbol;
use rand::Rng;

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
        #[clap(short, long,  default_value = None)]
        template: Option<AccountTemplate>,

        /// Executes a transaction that records the account on-chain
        #[clap(short, long, default_value_t = false)]
        deploy: bool,
    },
}

#[derive(Debug, Parser, Clone, ValueEnum)]
#[clap()]
pub enum AccountTemplate {
    /// Creates a basic account (Regular account with immutable code)
    BasicImmutable,
    /// Creates a faucet for fungible tokens
    FungibleFaucet,
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

pub fn list_accounts() -> Result<(), String> {
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

pub fn new_account(template: &Option<AccountTemplate>, deploy: bool) -> Result<(), String> {
    let client = Client::new(ClientConfig::default()).unwrap();

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

    let (account, _) = match template {
        None => todo!("Generic account creation is not supported yet"),
        Some(AccountTemplate::BasicImmutable) => {
            wallets::create_basic_wallet(init_seed, auth_scheme)
        }
        Some(AccountTemplate::FungibleFaucet) => faucets::create_basic_faucet(
            init_seed,
            TokenSymbol::new("TEST")
                .expect("Hardcoded test token symbol creation should not panic"),
            4u8,
            Felt::new(100u64),
            auth_scheme,
        ),
    }
    .map_err(|err| err.to_string())?;

    // TODO: as the client takes form, make errors more structured
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

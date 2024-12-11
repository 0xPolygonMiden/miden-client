use clap::{Parser, ValueEnum};
use miden_client::{
    accounts::{AccountStorageMode, AccountTemplate},
    assets::TokenSymbol,
    crypto::FeltRng,
    Client,
};

use crate::{
    commands::account::maybe_set_default_account, utils::load_config_file, CLIENT_BINARY_NAME,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliAccountStorageMode {
    Private,
    Public,
}

impl From<CliAccountStorageMode> for AccountStorageMode {
    fn from(cli_mode: CliAccountStorageMode) -> Self {
        match cli_mode {
            CliAccountStorageMode::Private => AccountStorageMode::Private,
            CliAccountStorageMode::Public => AccountStorageMode::Public,
        }
    }
}

#[derive(Debug, Parser, Clone)]
/// Create a new faucet account
pub struct NewFaucetCmd {
    #[clap(value_enum, short, long, default_value_t = CliAccountStorageMode::Private)]
    /// Storage mode of the account
    storage_mode: CliAccountStorageMode,
    #[clap(short, long)]
    /// Defines if the account assets are non-fungible (by default it is fungible)
    non_fungible: bool,
    #[clap(short, long)]
    /// Token symbol of the faucet
    token_symbol: Option<String>,
    #[clap(short, long)]
    /// Decimals of the faucet
    decimals: Option<u8>,
    #[clap(short, long)]
    max_supply: Option<u64>,
}

impl NewFaucetCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), String> {
        if self.non_fungible {
            todo!("Non-fungible faucets are not supported yet");
        }

        if self.token_symbol.is_none() || self.decimals.is_none() || self.max_supply.is_none() {
            return Err(
                "`token-symbol`, `decimals` and `max-supply` flags must be provided for a fungible faucet"
                    .to_string(),
            );
        }

        let decimals = self.decimals.expect("decimals must be provided");
        let token_symbol = self.token_symbol.clone().expect("token symbol must be provided");

        let client_template = AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new(token_symbol.as_str())
                .map_err(|err| format!("error: token symbol is invalid: {}", err))?,
            decimals,
            max_supply: self.max_supply.expect("max supply must be provided"),
            storage_mode: self.storage_mode.into(),
        };

        let (new_account, _account_seed) = client.new_account(client_template).await?;
        println!("Succesfully created new faucet.");
        println!(
            "To view account details execute `{CLIENT_BINARY_NAME} account -s {}`",
            new_account.id()
        );

        Ok(())
    }
}

#[derive(Debug, Parser, Clone)]
/// Create a new wallet account
pub struct NewWalletCmd {
    #[clap(value_enum, short, long, default_value_t = CliAccountStorageMode::Private)]
    /// Storage mode of the account
    pub storage_mode: CliAccountStorageMode,
    #[clap(short, long)]
    /// Defines if the account code is mutable (by default it isn't mutable)
    pub mutable: bool,
}

impl NewWalletCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), String> {
        let client_template = AccountTemplate::BasicWallet {
            mutable_code: self.mutable,
            storage_mode: self.storage_mode.into(),
        };

        let (new_account, _account_seed) = client.new_account(client_template).await?;
        println!("Succesfully created new wallet.");
        println!(
            "To view account details execute `{CLIENT_BINARY_NAME} account -s {}`",
            new_account.id()
        );

        let (mut current_config, _) = load_config_file()?;
        maybe_set_default_account(&mut current_config, new_account.id())?;

        Ok(())
    }
}

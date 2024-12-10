use clap::{Parser, ValueEnum};
use miden_client::{
    accounts::{AccountStorageMode, AccountType},
    assets::TokenSymbol,
    auth::{AuthScheme, AuthSecretKey},
    crypto::{FeltRng, SecretKey},
    utils::{create_basic_fungible_faucet, create_basic_wallet},
    Client, Felt,
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

        let key_pair = SecretKey::with_rng(client.rng());

        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key: key_pair.public_key() };

        let mut init_seed = [0u8; 32];
        client.rng().fill_bytes(&mut init_seed);

        let (new_account, seed) = create_basic_fungible_faucet(
            init_seed,
            TokenSymbol::new(token_symbol.as_str())
                .map_err(|err| format!("error: token symbol is invalid: {}", err))?,
            decimals,
            Felt::try_from(
                self.max_supply.expect("max supply must be provided").to_le_bytes().as_slice(),
            )
            .expect("u64 can be safely converted to a field element"),
            self.storage_mode.into(),
            auth_scheme,
        )
        .map_err(|err| format!("error: failed to create faucet: {}", err))?;

        client
            .insert_account(&new_account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair))
            .await?;

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
    /// Defines if the account code is mutable (by default it is not mutable)
    pub mutable: bool,
}

impl NewWalletCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), String> {
        let key_pair = SecretKey::with_rng(client.rng());

        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key: key_pair.public_key() };

        let mut init_seed = [0u8; 32];
        client.rng().fill_bytes(&mut init_seed);

        let account_type = if self.mutable {
            AccountType::RegularAccountUpdatableCode
        } else {
            AccountType::RegularAccountImmutableCode
        };

        let (new_account, seed) =
            create_basic_wallet(init_seed, auth_scheme, account_type, self.storage_mode.into())
                .map_err(|err| format!("error: failed to create wallet: {}", err))?;

        client
            .insert_account(&new_account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair))
            .await?;

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

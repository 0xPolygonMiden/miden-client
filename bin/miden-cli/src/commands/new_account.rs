use clap::{Parser, ValueEnum};
use miden_client::{
    account::{
        AccountBuilder, AccountStorageMode, AccountType, BasicFungibleFaucetComponent,
        BasicWalletComponent, RpoFalcon512Component,
    },
    assets::TokenSymbol,
    auth::AuthSecretKey,
    crypto::{FeltRng, SecretKey},
    Client, Felt,
};

use crate::{
    commands::account::maybe_set_default_account, errors::CliError, utils::load_config_file,
    CLIENT_BINARY_NAME,
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
/// Create a new faucet account.
pub struct NewFaucetCmd {
    #[clap(value_enum, short, long, default_value_t = CliAccountStorageMode::Private)]
    /// Storage mode of the account.
    storage_mode: CliAccountStorageMode,
    #[clap(short, long)]
    /// Defines if the account assets are non-fungible (by default it is fungible).
    non_fungible: bool,
    #[clap(short, long)]
    /// Token symbol of the faucet.
    token_symbol: Option<String>,
    #[clap(short, long)]
    /// Decimals of the faucet.
    decimals: Option<u8>,
    #[clap(short, long)]
    max_supply: Option<u64>,
}

impl NewFaucetCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), CliError> {
        if self.non_fungible {
            todo!("Non-fungible faucets are not supported yet");
        }

        if self.token_symbol.is_none() || self.decimals.is_none() || self.max_supply.is_none() {
            return Err(CliError::MissingFlag(
                "`token-symbol`, `decimals` and `max-supply` flags must be provided for a fungible faucet"
                    .to_string(),
            ));
        }

        let decimals = self.decimals.expect("decimals must be provided");
        let token_symbol = self.token_symbol.clone().expect("token symbol must be provided");

        let key_pair = SecretKey::with_rng(client.rng());

        let mut init_seed = [0u8; 32];
        client.rng().fill_bytes(&mut init_seed);

        let symbol = TokenSymbol::new(token_symbol.as_str()).map_err(CliError::Asset)?;
        let max_supply = Felt::try_from(
            self.max_supply.expect("max supply must be provided").to_le_bytes().as_slice(),
        )
        .expect("u64 can be safely converted to a field element");

        let anchor_block = client.get_latest_epoch_block().await?;

        let (new_account, seed) = AccountBuilder::new(init_seed)
            .anchor((&anchor_block).try_into().expect("anchor block should be valid"))
            .account_type(AccountType::FungibleFaucet)
            .storage_mode(self.storage_mode.into())
            .with_component(RpoFalcon512Component::new(key_pair.public_key()))
            .with_component(
                BasicFungibleFaucetComponent::new(symbol, decimals, max_supply).map_err(|err| {
                    CliError::Account(err, "Failed to create a faucet".to_string())
                })?,
            )
            .build()
            .map_err(|err| CliError::Account(err, "Failed to create a faucet".to_string()))?;

        client
            .add_account(&new_account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair), false)
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
/// Create a new wallet account.
pub struct NewWalletCmd {
    #[clap(value_enum, short, long, default_value_t = CliAccountStorageMode::Private)]
    /// Storage mode of the account.
    pub storage_mode: CliAccountStorageMode,
    #[clap(short, long)]
    /// Defines if the account code is mutable (by default it isn't mutable).
    pub mutable: bool,
}

impl NewWalletCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), CliError> {
        let key_pair = SecretKey::with_rng(client.rng());

        let mut init_seed = [0u8; 32];
        client.rng().fill_bytes(&mut init_seed);

        let account_type = if self.mutable {
            AccountType::RegularAccountUpdatableCode
        } else {
            AccountType::RegularAccountImmutableCode
        };

        let anchor_block = client.get_latest_epoch_block().await?;

        let (new_account, seed) = AccountBuilder::new(init_seed)
            .anchor((&anchor_block).try_into().expect("anchor block should be valid"))
            .account_type(account_type)
            .storage_mode(self.storage_mode.into())
            .with_component(RpoFalcon512Component::new(key_pair.public_key()))
            .with_component(BasicWalletComponent)
            .build()
            .map_err(|err| CliError::Account(err, "Failed to create a wallet".to_string()))?;

        client
            .add_account(&new_account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair), false)
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

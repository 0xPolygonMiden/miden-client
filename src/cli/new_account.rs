use clap::{Parser, ValueEnum};
use miden_client::{
    client::{accounts, rpc::NodeRpcClient, Client},
    store::Store,
};
use miden_objects::{assets::TokenSymbol, crypto::rand::FeltRng};
use miden_tx::TransactionAuthenticator;

use crate::cli::CLIENT_BINARY_NAME;

#[derive(Debug, Parser, Clone)]
/// Create a new faucet account and store it locally
pub struct NewFaucetCmd {
    #[clap(short, long, value_enum, default_value_t = AccountStorageMode::OffChain)]
    /// Storage type of the account
    pub storage_type: AccountStorageMode,
    #[clap(short, long)]
    /// Defines if the account assets are non-fungible (by default it is fungible)
    pub non_fungible: bool,
    #[clap(short, long)]
    /// Token symbol of the faucet
    token_symbol: Option<String>,
    #[clap(short, long)]
    /// Decimals of the faucet
    decimals: Option<u8>,
    #[clap(short, long)]
    /// Max supply of the faucet
    max_supply: Option<u64>,
}

impl NewFaucetCmd {
    pub fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        mut client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        if self.non_fungible {
            return Err("Non-fungible faucets are not supported yet".to_string());
        }

        if self.token_symbol.is_none() || self.decimals.is_none() || self.max_supply.is_none() {
            return Err(
                "Token symbol, decimals and max supply must be provided for a fungible faucet"
                    .to_string(),
            );
        }

        let client_template = accounts::AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new(
                self.token_symbol.clone().expect("token symbol must be provided").as_str(),
            )
            .map_err(|err| format!("error: token symbol is invalid: {}", err))?,
            decimals: self.decimals.expect("decimals must be provided"),
            max_supply: self.max_supply.expect("max supply must be provided"),
            storage_mode: self.storage_type.into(),
        };

        let (new_account, _account_seed) = client.new_account(client_template)?;
        println!("Succesfully created new faucet.");
        println!(
            "To view account details execute `{CLIENT_BINARY_NAME} account -s {}`",
            new_account.id()
        );

        Ok(())
    }
}

#[derive(Debug, Parser, Clone)]
/// Create a new wallet account and store it locally
pub struct NewWalletCmd {
    #[clap(short, long, value_enum, default_value_t = AccountStorageMode::OffChain)]
    /// Storage type of the account
    pub storage_type: AccountStorageMode,
    #[clap(short, long)]
    /// Defines if the account code is mutable (by default it is not mutable)
    pub mutable: bool,
}

impl NewWalletCmd {
    pub fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        mut client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        let client_template = accounts::AccountTemplate::BasicWallet {
            mutable_code: self.mutable,
            storage_mode: self.storage_type.into(),
        };

        let (new_account, _account_seed) = client.new_account(client_template)?;
        println!("Succesfully created new wallet.");
        println!(
            "To view account details execute `{CLIENT_BINARY_NAME} account -s {}`",
            new_account.id()
        );

        Ok(())
    }
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

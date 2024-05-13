use clap::{Parser, ValueEnum};
use miden_client::{
    client::{accounts, rpc::NodeRpcClient, Client},
    store::Store,
};
use miden_objects::{assets::TokenSymbol, crypto::rand::FeltRng};
use miden_tx::TransactionAuthenticator;

use crate::cli::CLIENT_BINARY_NAME;

#[derive(Debug, Parser, Clone)]
pub struct NewAccountCmd {
    #[clap(short, long, value_enum)]
    pub acc_type: AccountTemplate,
    #[clap(short, long, value_enum, default_value_t = AccountStorageMode::OffChain)]
    pub storage_type: AccountStorageMode,
    #[clap(short, long)]
    token_symbol: Option<String>,
    #[clap(short, long)]
    decimals: Option<u8>,
    #[clap(short, long)]
    max_supply: Option<u64>,
}

impl NewAccountCmd {
    pub fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        mut client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        let client_template = match self.acc_type {
            AccountTemplate::BasicImmutable => accounts::AccountTemplate::BasicWallet {
                mutable_code: false,
                storage_mode: self.storage_type.into(),
            },
            AccountTemplate::BasicMutable => accounts::AccountTemplate::BasicWallet {
                mutable_code: true,
                storage_mode: self.storage_type.into(),
            },
            AccountTemplate::FungibleFaucet => {
                if self.token_symbol.is_none()
                    || self.decimals.is_none()
                    || self.max_supply.is_none()
                {
                    return Err("Token symbol, decimals and max supply must be provided for a fungible faucet".to_string());
                }

                accounts::AccountTemplate::FungibleFaucet {
                    token_symbol: TokenSymbol::new(
                        self.token_symbol.clone().expect("token symbol must be provided").as_str(),
                    )
                    .map_err(|err| format!("error: token symbol is invalid: {}", err))?,
                    decimals: self.decimals.expect("decimals must be provided"),
                    max_supply: self.max_supply.expect("max supply must be provided"),
                    storage_mode: self.storage_type.into(),
                }
            },
            AccountTemplate::NonFungibleFaucet => todo!(),
        };
        let (new_account, _account_seed) = client.new_account(client_template)?;
        println!("Succesfully created new account.");
        println!(
            "To view account details execute `{CLIENT_BINARY_NAME} account -s {}`",
            new_account.id()
        );

        Ok(())
    }
}

#[derive(Debug, Parser, Clone, ValueEnum)]
pub enum AccountTemplate {
    /// Creates a basic account (Regular account with immutable code)
    BasicImmutable,
    /// Creates a basic account (Regular account with mutable code)
    BasicMutable,
    /// Creates a faucet for fungible tokens
    FungibleFaucet,
    /// Creates a faucet for non-fungible tokens
    NonFungibleFaucet,
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

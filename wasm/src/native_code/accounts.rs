use web_sys::console;
use wasm_bindgen::prelude::*;

use miden_lib::AuthScheme;
use miden_objects::{
    accounts::{
        Account, AccountData, AccountId, AccountStorageType, AccountStub, AccountType, AuthData 
    }, assets::TokenSymbol, crypto::{
        dsa::rpo_falcon512::SecretKey,
        rand::{FeltRng, RpoRandomCoin},
    }, Digest, Word
};

use crate::native_code::store::AuthInfo;
use crate::native_code::errors::ClientError;

use super::{
    rpc::NodeRpcClient, 
    Client, 
    store::Store // TODO: Add AuthInfo
};

pub enum AccountTemplate {
    BasicWallet {
        mutable_code: bool,
        storage_mode: AccountStorageMode,
    },
    FungibleFaucet {
        token_symbol: TokenSymbol,
        decimals: u8,
        max_supply: u64,
        storage_mode: AccountStorageMode,
    },
}

// TODO: Review this enum and variant names to have a consistent naming across all crates
#[derive(Debug, Clone, Copy)]
pub enum AccountStorageMode {
    Local,
    OnChain,
}

impl From<AccountStorageMode> for AccountStorageType {
    fn from(mode: AccountStorageMode) -> Self {
        match mode {
            AccountStorageMode::Local => AccountStorageType::OffChain,
            AccountStorageMode::OnChain => AccountStorageType::OnChain,
        }
    }
}

impl<N: NodeRpcClient, R: FeltRng, S: Store> Client<N, R, S> {
    // ACCOUNT CREATION
    // --------------------------------------------------------------------------------------------

    /// Creates a new [Account] based on an [AccountTemplate] and saves it in the store
    pub async fn new_account(
        &mut self,
        template: AccountTemplate,
    ) -> Result<(Account, Word), ClientError> {
        let account_and_seed = match template {
            AccountTemplate::BasicWallet {
                mutable_code,
                storage_mode,
            } => self.new_basic_wallet(mutable_code,storage_mode).await,
            AccountTemplate::FungibleFaucet {
                token_symbol,
                decimals,
                max_supply,
                storage_mode,
            } => {
                todo!();
                //self.new_fungible_faucet(token_symbol, decimals, max_supply, storage_mode).await
            }
        }?;

        Ok(account_and_seed)
    }

    /// Creates a new regular account and saves it in the store along with its seed and auth data
    ///
    /// # Panics
    ///
    /// If the passed [AccountStorageMode] is [AccountStorageMode::OnChain], this function panics
    /// since this feature is not currently supported on Miden
    async fn new_basic_wallet(
        &mut self,
        mutable_code: bool,
        account_storage_mode: AccountStorageMode,
    ) -> Result<(Account, Word), ClientError>  {
        let key_pair = SecretKey::with_rng(&mut self.rng);

        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key: key_pair.public_key() };

        // we need to use an initial seed to create the wallet account
        let mut init_seed = [0u8; 32];
        self.rng.fill_bytes(&mut init_seed);

        let (account, seed) = if !mutable_code {
            miden_lib::accounts::wallets::create_basic_wallet(
                init_seed,
                auth_scheme,
                AccountType::RegularAccountImmutableCode,
                account_storage_mode.into(),
            )
        } else {
            miden_lib::accounts::wallets::create_basic_wallet(
                init_seed,
                auth_scheme,
                AccountType::RegularAccountUpdatableCode,
                account_storage_mode.into(),
            )
        }?;

        let _ = self.insert_account(&account, Some(seed), &AuthInfo::RpoFalcon512(key_pair)).await;

        Ok((account, seed))
    }

    async fn new_fungible_faucet(
        &mut self,
        token_symbol: TokenSymbol,
        decimals: u8,
        max_supply: u64,
        account_storage_mode: AccountStorageMode,
    ) -> String{ // TODO: Replace with Result<(Account, Word), ()>

        // if let AccountStorageMode::OnChain = account_storage_mode {
        //     todo!("On-chain accounts are not supported yet");
        // }

        // let key_pair: KeyPair = KeyPair::new()?;

        // let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 {
        //     pub_key: key_pair.public_key(),
        // };

        // // we need to use an initial seed to create the wallet account
        // let init_seed: [u8; 32] = rng.gen();

        // let (account, seed) = miden_lib::accounts::faucets::create_basic_fungible_faucet(
        //     init_seed,
        //     token_symbol,
        //     decimals,
        //     Felt::try_from(max_supply.to_le_bytes().as_slice())
        //         .expect("u64 can be safely converted to a field element"),
        //     auth_scheme,
        // )?;

        // self.insert_account(&account, Some(seed), &AuthInfo::RpoFalcon512(key_pair))?;
        // Ok((account, seed))

        "Called new_fungible_faucet".to_string()
    }

    pub async fn import_account(
        &mut self, 
        account_data: AccountData
    ) -> Result<(), ()> {
        match account_data.auth {
            AuthData::RpoFalcon512Seed(key_pair_seed) => {
                // NOTE: The seed should probably come from a different format from miden-base's AccountData
                let seed = Digest::try_from(&key_pair_seed).unwrap().into();
                let mut rng = RpoRandomCoin::new(seed);

                let key_pair = SecretKey::with_rng(&mut rng);

                let account_seed = if !account_data.account.is_new()
                    && account_data.account_seed.is_some()
                {
                    tracing::warn!("Imported an existing account and still provided a seed when it is not needed. It's possible that the account's file was incorrectly generated. The seed will be ignored.");
                    // Ignore the seed since it's not a new account

                    // TODO: The alternative approach to this is to store the seed anyway, but
                    // ignore it at the point of executing against this transaction, but that
                    // approach seems a little bit more incorrect
                    None
                } else {
                    account_data.account_seed
                };

                self.insert_account(
                    &account_data.account,
                    account_seed,
                    &AuthInfo::RpoFalcon512(key_pair),
                ).await
            },
        }
    }

    pub async fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthInfo,
    ) -> Result<(), ()> {
        if account.is_new() && account_seed.is_none() {
            return Err(());
        }

        self.store
            .insert_account(account, account_seed, auth_info).await
    }

    pub async fn get_accounts(
        &mut self
    ) -> Result<Vec<(AccountStub, Option<Word>)>, ()> {
        self.store.get_account_stubs().await
    }

    pub async fn get_account(
        &mut self,
        account_id: AccountId
    ) -> Result<(Account, Option<Word>), ()> {
        self.store.get_account(account_id).await
    }

    pub async fn get_account_stub_by_id(
        &mut self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), ()> {
        self.store.get_account_stub(account_id).await
    }

    pub async fn get_account_auth(
        &mut self,
        account_id: AccountId
    ) -> Result<AuthInfo, ()> {
        self.store.get_account_auth(account_id).await
    }
}

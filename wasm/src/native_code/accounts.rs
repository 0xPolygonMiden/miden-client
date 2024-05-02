use web_sys::console;
use wasm_bindgen::prelude::*;

use miden_lib::AuthScheme;
use miden_objects::{
    accounts::{
        Account, AccountData, AccountId, AccountStorageType, AccountStub, AccountType, AuthData 
    }, assets::TokenSymbol, crypto::{
        dsa::rpo_falcon512::SecretKey,
        rand::{
            FeltRng, RpoRandomCoin
        },
    }, Digest, Felt, Word
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
                self.new_fungible_faucet(token_symbol, decimals, max_supply, storage_mode).await
            }
        }?;

        Ok(account_and_seed)
    }

    /// Saves in the store the [Account] corresponding to `account_data`.
    ///
    /// # Errors
    ///
    /// Will return an error if trying to import a new account without providing its seed
    ///
    /// # Panics
    ///
    /// Will panic when trying to import a non-new account without a seed since this functionality
    /// is not currently implemented
    pub async fn import_account(
        &mut self, 
        account_data: AccountData
    ) -> Result<(), ClientError> {
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

    /// Creates a new regular account and saves it in the store along with its seed and auth data
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

        self.insert_account(&account, Some(seed), &AuthInfo::RpoFalcon512(key_pair)).await?;

        Ok((account, seed))
    }

    async fn new_fungible_faucet(
        &mut self,
        token_symbol: TokenSymbol,
        decimals: u8,
        max_supply: u64,
        account_storage_mode: AccountStorageMode,
    ) -> Result<(Account, Word), ClientError> {
        let key_pair = SecretKey::with_rng(&mut self.rng);

        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key: key_pair.public_key() };

        // we need to use an initial seed to create the wallet account
        let mut init_seed = [0u8; 32];
        self.rng.fill_bytes(&mut init_seed);

        let (account, seed) = miden_lib::accounts::faucets::create_basic_fungible_faucet(
            init_seed,
            token_symbol,
            decimals,
            Felt::try_from(max_supply.to_le_bytes().as_slice())
                .expect("u64 can be safely converted to a field element"),
            account_storage_mode.into(),
            auth_scheme,
        )?;

        self.insert_account(&account, Some(seed), &AuthInfo::RpoFalcon512(key_pair)).await?;
        Ok((account, seed))
    }

    /// Inserts a new account into the client's store.
    ///
    /// # Errors
    ///
    /// If an account is new and no seed is provided, the function errors out because the client
    /// cannot execute transactions against new accounts for which it does not know the seed.
    pub async fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthInfo,
    ) -> Result<(), ClientError> {
        if account.is_new() && account_seed.is_none() {
            return Err(ClientError::ImportNewAccountWithoutSeed);
        }

        self.store
            .insert_account(account, account_seed, auth_info).await
            .map_err(ClientError::StoreError)
    }

    // ACCOUNT DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns summary info about the accounts managed by this client.
    pub async fn get_accounts(&self) -> Result<Vec<(AccountStub, Option<Word>)>, ClientError> {
        self.store.get_account_stubs().await.map_err(|err| err.into())
    }

    /// Returns summary info about the specified account.
    pub async fn get_account(
        &self,
        account_id: AccountId
    ) -> Result<(Account, Option<Word>), ClientError> {
        self.store.get_account(account_id).await.map_err(|err| err.into())
    }

    /// Returns summary info about the specified account.
    pub async fn get_account_stub_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), ClientError> {
        self.store.get_account_stub(account_id).await.map_err(|err| err.into())
    }

    /// Returns an [AuthInfo] object utilized to authenticate an account.
    ///
    /// # Errors
    ///
    /// Returns a [ClientError::StoreError] with a [StoreError::AccountDataNotFound](crate::errors::StoreError::AccountDataNotFound) if the provided ID does
    /// not correspond to an existing account.
    pub async fn get_account_auth(
        &self,
        account_id: AccountId
    ) -> Result<AuthInfo, ClientError> {
        self.store.get_account_auth(account_id).await.map_err(|err| err.into())
    }
}

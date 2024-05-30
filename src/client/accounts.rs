use miden_lib::AuthScheme;
use miden_objects::{
    accounts::{
        Account, AccountData, AccountId, AccountStorageType, AccountStub, AccountType,
        AuthSecretKey,
    },
    assets::TokenSymbol,
    crypto::{dsa::rpo_falcon512::SecretKey, rand::FeltRng},
    Felt, Word,
};
use miden_tx::TransactionAuthenticator;

use super::{rpc::NodeRpcClient, Client};
use crate::{errors::ClientError, store::Store};

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

impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> Client<N, R, S, A> {
    // ACCOUNT CREATION
    // --------------------------------------------------------------------------------------------

    /// Creates a new [Account] based on an [AccountTemplate] and saves it in the store
    #[cfg(not(feature = "wasm"))]
    pub fn new_account(
        &mut self,
        template: AccountTemplate,
    ) -> Result<(Account, Word), ClientError> {
        let account_and_seed = match template {
            AccountTemplate::BasicWallet { mutable_code, storage_mode } => {
                self.new_basic_wallet(mutable_code, storage_mode)
            },
            AccountTemplate::FungibleFaucet {
                token_symbol,
                decimals,
                max_supply,
                storage_mode,
            } => self.new_fungible_faucet(token_symbol, decimals, max_supply, storage_mode),
        }?;

        Ok(account_and_seed)
    }

    #[cfg(feature = "wasm")]
    pub async fn new_account(
        &mut self,
        template: AccountTemplate,
    ) -> Result<(Account, Word), ClientError> {
        let account_and_seed = match template {
            AccountTemplate::BasicWallet { mutable_code, storage_mode } => {
                self.new_basic_wallet(mutable_code, storage_mode).await
            },
            AccountTemplate::FungibleFaucet {
                token_symbol,
                decimals,
                max_supply,
                storage_mode,
            } => self.new_fungible_faucet(token_symbol, decimals, max_supply, storage_mode).await,
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
    #[cfg(not(feature = "wasm"))]
    pub fn import_account(&mut self, account_data: AccountData) -> Result<(), ClientError> {
        let account_seed = if !account_data.account.is_new() && account_data.account_seed.is_some()
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

        self.insert_account(&account_data.account, account_seed, &account_data.auth_secret_key)
    }

    #[cfg(feature = "wasm")]
    pub async fn import_account(&mut self, account_data: AccountData) -> Result<(), ClientError> {
        let account_seed = if !account_data.account.is_new() && account_data.account_seed.is_some()
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

        self.insert_account(&account_data.account, account_seed, &account_data.auth_secret_key).await
    }

    /// Creates a new regular account and saves it in the store along with its seed and auth data
    #[cfg(not(feature = "wasm"))]
    fn new_basic_wallet(
        &mut self,
        mutable_code: bool,
        account_storage_mode: AccountStorageMode,
    ) -> Result<(Account, Word), ClientError> {
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

        self.insert_account(&account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair))?;
        Ok((account, seed))
    }

    #[cfg(feature = "wasm")]
    async fn new_basic_wallet(
        &mut self,
        mutable_code: bool,
        account_storage_mode: AccountStorageMode,
    ) -> Result<(Account, Word), ClientError> {
        let key_pair = SecretKey::with_rng(&mut self.rng());

        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key: key_pair.public_key() };

        // we need to use an initial seed to create the wallet account
        let mut init_seed = [0u8; 32];
        self.rng().fill_bytes(&mut init_seed);

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

        self.insert_account(&account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair)).await?;
        Ok((account, seed))
    }

    #[cfg(not(feature = "wasm"))]
    fn new_fungible_faucet(
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

        self.insert_account(&account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair))?;
        Ok((account, seed))
    }

    #[cfg(feature = "wasm")]
    async fn new_fungible_faucet(
        &mut self,
        token_symbol: TokenSymbol,
        decimals: u8,
        max_supply: u64,
        account_storage_mode: AccountStorageMode,
    ) -> Result<(Account, Word), ClientError> {
        let key_pair = SecretKey::with_rng(&mut self.rng());

        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key: key_pair.public_key() };

        // we need to use an initial seed to create the wallet account
        let mut init_seed = [0u8; 32];
        self.rng().fill_bytes(&mut init_seed);

        let (account, seed) = miden_lib::accounts::faucets::create_basic_fungible_faucet(
            init_seed,
            token_symbol,
            decimals,
            Felt::try_from(max_supply.to_le_bytes().as_slice())
                .expect("u64 can be safely converted to a field element"),
            account_storage_mode.into(),
            auth_scheme,
        )?;

        self.insert_account(&account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair)).await?;
        Ok((account, seed))
    }

    /// Inserts a new account into the client's store.
    ///
    /// # Errors
    ///
    /// If an account is new and no seed is provided, the function errors out because the client
    /// cannot execute transactions against new accounts for which it does not know the seed.
    #[cfg(not(feature = "wasm"))]
    pub fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthSecretKey,
    ) -> Result<(), ClientError> {
        if account.is_new() && account_seed.is_none() {
            return Err(ClientError::ImportNewAccountWithoutSeed);
        }

        self.store
            .insert_account(account, account_seed, auth_info)
            .map_err(ClientError::StoreError)
    }

    #[cfg(feature = "wasm")]
    pub async fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthSecretKey,
    ) -> Result<(), ClientError> {
        if account.is_new() && account_seed.is_none() {
            return Err(ClientError::ImportNewAccountWithoutSeed);
        }

        self.store()
            .insert_account(account, account_seed, auth_info).await
            .map_err(ClientError::StoreError)
    }

    // ACCOUNT DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns summary info about the accounts managed by this client.
    #[cfg(not(feature = "wasm"))]
    pub fn get_account_stubs(&self) -> Result<Vec<(AccountStub, Option<Word>)>, ClientError> {
        self.store.get_account_stubs().map_err(|err| err.into())
    }

    #[cfg(feature = "wasm")]
    pub async fn get_account_stubs(&mut self) -> Result<Vec<(AccountStub, Option<Word>)>, ClientError> {
        self.store().get_account_stubs().await.map_err(|err| err.into())
    }

    /// Returns summary info about the specified account.
    #[cfg(not(feature = "wasm"))]
    pub fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<(Account, Option<Word>), ClientError> {
        self.store.get_account(account_id).map_err(|err| err.into())
    }

    #[cfg(feature = "wasm")]
    pub async fn get_account(
        &mut self,
        account_id: AccountId,
    ) -> Result<(Account, Option<Word>), ClientError> {
        self.store().get_account(account_id).await.map_err(|err| err.into())
    }

    /// Returns summary info about the specified account.
    #[cfg(not(feature = "wasm"))]
    pub fn get_account_stub_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), ClientError> {
        self.store.get_account_stub(account_id).map_err(|err| err.into())
    }

    /// Returns summary info about the specified account.
    #[cfg(feature = "wasm")]
    pub async fn get_account_stub_by_id(
        &mut self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), ClientError> {
        self.store().get_account_stub(account_id).await.map_err(|err| err.into())
    }

    /// Returns an [AuthSecretKey] object utilized to authenticate an account.
    ///
    /// # Errors
    ///
    /// Returns a [ClientError::StoreError] with a [StoreError::AccountDataNotFound](crate::errors::StoreError::AccountDataNotFound) if the provided ID does
    /// not correspond to an existing account.
    #[cfg(not(feature = "wasm"))]
    pub fn get_account_auth(&self, account_id: AccountId) -> Result<AuthSecretKey, ClientError> {
        self.store.get_account_auth(account_id).map_err(|err| err.into())
    }

    #[cfg(feature = "wasm")]
    pub async fn get_account_auth(&mut self, account_id: AccountId) -> Result<AuthSecretKey, ClientError> {
        self.store().get_account_auth(account_id).await.map_err(|err| err.into())
    }
}

// TESTS
// ================================================================================================

#[cfg(all(test, not(feature = "wasm")))]
pub mod tests {
    use miden_objects::{
        accounts::{Account, AccountData, AccountId, AuthSecretKey},
        crypto::dsa::rpo_falcon512::SecretKey,
        Word,
    };

    use crate::mock::{
        create_test_client, get_account_with_default_account_code,
        get_new_account_with_default_account_code, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_REGULAR,
    };

    fn create_account_data(account_id: u64) -> AccountData {
        let account_id = AccountId::try_from(account_id).unwrap();
        let account = get_account_with_default_account_code(account_id, Word::default(), None);

        AccountData::new(
            account.clone(),
            Some(Word::default()),
            AuthSecretKey::RpoFalcon512(SecretKey::new()),
        )
    }

    pub fn create_initial_accounts_data() -> Vec<AccountData> {
        let account = create_account_data(ACCOUNT_ID_REGULAR);

        let faucet_account = create_account_data(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN);

        // Create Genesis state and save it to a file
        let accounts = vec![account, faucet_account];

        accounts
    }

    #[test]
    pub fn try_import_new_account() {
        // generate test client
        let mut client = create_test_client();

        let account = get_new_account_with_default_account_code(
            AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap(),
            Word::default(),
            None,
        );

        let key_pair = SecretKey::new();

        assert!(client
            .insert_account(&account, None, &AuthSecretKey::RpoFalcon512(key_pair.clone()))
            .is_err());
        assert!(client
            .insert_account(&account, Some(Word::default()), &AuthSecretKey::RpoFalcon512(key_pair))
            .is_ok());
    }

    #[tokio::test]
    async fn load_accounts_test() {
        // generate test client
        let mut client = create_test_client();

        let created_accounts_data = create_initial_accounts_data();

        for account_data in created_accounts_data.clone() {
            client.import_account(account_data).unwrap();
        }

        let expected_accounts: Vec<Account> = created_accounts_data
            .into_iter()
            .map(|account_data| account_data.account)
            .collect();
        let accounts = client.get_account_stubs().unwrap();

        assert_eq!(accounts.len(), 2);
        for (client_acc, expected_acc) in accounts.iter().zip(expected_accounts.iter()) {
            assert_eq!(client_acc.0.hash(), expected_acc.hash());
        }
    }
}

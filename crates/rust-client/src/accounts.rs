//! The `accounts` module provides types and client APIs for managing accounts within the Miden
//! rollup network .
//!
//! Accounts can be created or imported. Once they are tracked by the client, their state will be
//! updated accordingly on every transaction, and validated against the rollup on every sync.

use alloc::vec::Vec;

use miden_lib::AuthScheme;
pub use miden_objects::accounts::{
    Account, AccountCode, AccountData, AccountHeader, AccountId, AccountStorage,
    AccountStorageMode, AccountType, StorageSlot, StorageSlotType,
};
use miden_objects::{
    accounts::AuthSecretKey,
    assets::TokenSymbol,
    crypto::{dsa::rpo_falcon512::SecretKey, rand::FeltRng},
    Felt, Word,
};

use super::Client;
use crate::ClientError;

/// Defines templates for creating different types of Miden accounts.
pub enum AccountTemplate {
    /// The `BasicWallet` variant represents a regular wallet account.
    BasicWallet {
        /// A boolean indicating whether the account's code can be modified after creation.
        mutable_code: bool,
        /// Specifies the type of storage used by the account. This is defined by the
        /// `AccountStorageMode` enum.
        storage_mode: AccountStorageMode,
    },

    /// The `FungibleFaucet` variant represents an account designed to issue fungible tokens.
    FungibleFaucet {
        /// The symbol of the token being issued by the faucet.
        token_symbol: TokenSymbol,
        /// The number of decimal places used by the token.
        decimals: u8,
        /// The maximum supply of tokens that the faucet can issue.
        max_supply: u64,
        /// Specifies the type of storage used by the account.
        storage_mode: AccountStorageMode,
    },
}

impl<R: FeltRng> Client<R> {
    // ACCOUNT CREATION
    // --------------------------------------------------------------------------------------------

    /// Creates a new [Account] based on an [AccountTemplate] and saves it in the client's store. A
    /// new tag derived from the account will start being tracked by the client.
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

        self.store.add_note_tag((&account_and_seed.0).try_into()?).await?;

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

        self.insert_account(&account_data.account, account_seed, &account_data.auth_secret_key)
            .await
    }

    /// Creates a new regular account and saves it in the store along with its seed and auth data
    async fn new_basic_wallet(
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
                account_storage_mode,
            )
        } else {
            miden_lib::accounts::wallets::create_basic_wallet(
                init_seed,
                auth_scheme,
                AccountType::RegularAccountUpdatableCode,
                account_storage_mode,
            )
        }?;

        self.insert_account(&account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair))
            .await?;
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
            account_storage_mode,
            auth_scheme,
        )?;

        self.insert_account(&account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair))
            .await?;
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
        auth_info: &AuthSecretKey,
    ) -> Result<(), ClientError> {
        if account.is_new() && account_seed.is_none() {
            return Err(ClientError::ImportNewAccountWithoutSeed);
        }

        self.store
            .insert_account(account, account_seed, auth_info)
            .await
            .map_err(ClientError::StoreError)
    }

    // ACCOUNT DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns a list of [AccountHeader] of all accounts stored in the database along with the
    /// seeds used to create them.
    ///
    /// Said accounts' state is the state after the last performed sync.
    pub async fn get_account_headers(
        &self,
    ) -> Result<Vec<(AccountHeader, Option<Word>)>, ClientError> {
        self.store.get_account_headers().await.map_err(|err| err.into())
    }

    /// Retrieves a full [Account] object. The seed will be returned if the account is new,
    /// otherwise it will be `None`.
    ///
    /// This function returns the [Account]'s latest state. If the account is new (that is, has
    /// never executed a transaction), the returned seed will be `Some(Word)`; otherwise the seed
    /// will be `None`
    ///
    /// # Errors
    ///
    /// Returns a `StoreError::AccountDataNotFound` if there is no account for the provided ID
    pub async fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<(Account, Option<Word>), ClientError> {
        self.store.get_account(account_id).await.map_err(|err| err.into())
    }

    /// Retrieves an [AccountHeader] object for the specified [AccountId] along with the seed
    /// used to create it. The seed will be returned if the account is new, otherwise it
    /// will be `None`.
    ///
    /// Said account's state is the state according to the last sync performed.
    ///
    /// # Errors
    ///
    /// Returns a `StoreError::AccountDataNotFound` if there is no account for the provided ID
    pub async fn get_account_header_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountHeader, Option<Word>), ClientError> {
        self.store.get_account_header(account_id).await.map_err(|err| err.into())
    }

    /// Returns an [AuthSecretKey] object utilized to authenticate an account.
    ///
    /// # Errors
    ///
    /// Returns a [ClientError::StoreError] with a
    /// [StoreError::AccountDataNotFound](crate::store::StoreError::AccountDataNotFound) if the
    /// provided ID does not correspond to an existing account.
    pub async fn get_account_auth(
        &self,
        account_id: AccountId,
    ) -> Result<AuthSecretKey, ClientError> {
        self.store.get_account_auth(account_id).await.map_err(|err| err.into())
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use alloc::vec::Vec;

    use miden_lib::transaction::TransactionKernel;
    use miden_objects::{
        accounts::{
            account_id::testing::{
                ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            },
            Account, AccountData, AuthSecretKey,
        },
        crypto::dsa::rpo_falcon512::SecretKey,
        Felt, Word,
    };

    use crate::mock::create_test_client;

    fn create_account_data(account_id: u64) -> AccountData {
        let account =
            Account::mock(account_id, Felt::new(2), TransactionKernel::testing_assembler());

        AccountData::new(
            account.clone(),
            Some(Word::default()),
            AuthSecretKey::RpoFalcon512(SecretKey::new()),
        )
    }

    pub fn create_initial_accounts_data() -> Vec<AccountData> {
        let account = create_account_data(ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN);

        let faucet_account = create_account_data(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN);

        // Create Genesis state and save it to a file
        let accounts = vec![account, faucet_account];

        accounts
    }

    #[tokio::test]
    pub async fn try_import_new_account() {
        // generate test client
        let (mut client, _rpc_api) = create_test_client().await;

        let account = Account::mock(
            ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN,
            Felt::new(0),
            TransactionKernel::testing_assembler(),
        );

        let key_pair = SecretKey::new();

        assert!(client
            .insert_account(&account, None, &AuthSecretKey::RpoFalcon512(key_pair.clone()))
            .await
            .is_err());
        assert!(client
            .insert_account(&account, Some(Word::default()), &AuthSecretKey::RpoFalcon512(key_pair))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn load_accounts_test() {
        // generate test client
        let (mut client, _) = create_test_client().await;

        let created_accounts_data = create_initial_accounts_data();

        for account_data in created_accounts_data.clone() {
            client.import_account(account_data).await.unwrap();
        }

        let expected_accounts: Vec<Account> = created_accounts_data
            .into_iter()
            .map(|account_data| account_data.account)
            .collect();
        let accounts = client.get_account_headers().await.unwrap();

        assert_eq!(accounts.len(), 2);
        for (client_acc, expected_acc) in accounts.iter().zip(expected_accounts.iter()) {
            assert_eq!(client_acc.0.hash(), expected_acc.hash());
        }
    }
}

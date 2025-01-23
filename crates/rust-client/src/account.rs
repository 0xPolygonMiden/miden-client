//! The `accounts` module provides types and client APIs for managing accounts within the Miden
//! rollup network.
//!
//! Once accounts start being tracked by the client, their state will be updated accordingly on
//! every transaction, and validated against the rollup on every sync.
//!
//! An account can store assets and define rules for manipulating them.
//!
//! # Overview
//!
//! This module exposes several key features:
//!
//! - **Account types:** Use the [`AccountBuilder`] to construct new accounts, specifying account
//!   type, storage mode (public/private), and attaching necessary components (e.g., basic wallet or
//!   fungible faucet).
//!
//! - **Account Tracking:** Accounts added via the client are persisted to the local store, where
//!   their state (including nonce, balance, and metadata) is updated upon every synchronization
//!   with the network.
//!
//! - **Data retrieval APIs:** The module also provides methods to fetch account-related data.
//!
//! # Example
//!
//! To add a new account to the client's store, you might use the [`Client::add_account`] method as
//! follows:
//!
//! ```rust
//! # use miden_client::accounts::{Account, AccountBuilder, AccountType, BasicWalletComponent};
//! # use miden_objects::account::{AuthSecretKey, AccountStorageMode};
//! # use miden_client::crypto::{FeltRng, SecretKey};
//! # async fn add_new_account_example(client: &mut miden_client::Client<impl FeltRng>) -> Result<(), miden_client::ClientError> {
//! let key_pair = SecretKey::with_rng(client.rng());
//! # let random_seed = Default::default();
//!
//! let (account, seed) = AccountBuilder::new(random_seed)
//!     .account_type(AccountType::RegularAccountImmutableCode)
//!     .storage_mode(AccountStorageMode::Private)
//!     .with_component(BasicWalletComponent)
//!     .build()?;
//!
//! // Add the account to the client. The account seed and authentication key are required for new accounts.
//! client.add_account(&account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair), false).await?;
//! # Ok(())
//! # }
//! ```
//!
//! For more details on accounts, refer to the [Account] documentation.

use alloc::vec::Vec;

pub use miden_lib::account::{
    auth::RpoFalcon512 as RpoFalcon512Component,
    faucets::BasicFungibleFaucet as BasicFungibleFaucetComponent,
    wallets::BasicWallet as BasicWalletComponent,
};
pub use miden_objects::account::{
    Account, AccountBuilder, AccountCode, AccountData, AccountHeader, AccountId, AccountStorage,
    AccountStorageMode, AccountType, StorageSlot, StorageSlotType,
};
use miden_objects::{account::AuthSecretKey, crypto::rand::FeltRng, Digest, Word};

use super::Client;
use crate::{
    store::{AccountRecord, AccountStatus},
    ClientError,
};

impl<R: FeltRng> Client<R> {
    // ACCOUNT CREATION
    // --------------------------------------------------------------------------------------------

    /// Adds the provided [Account] in the store so it can start being tracked by the client.
    ///
    /// If the account is already being tracked and `overwrite` is set to `true`, the account will
    /// be overwritten. The `account_seed` should be provided if the account is newly created.
    /// The `auth_secret_key` is stored in client but it is never exposed. It is used to
    /// authenticate transactions against the account. The seed is used when notifying the
    /// network about a new account and is not used for any other purpose.
    ///
    /// # Errors
    ///
    /// - Trying to import a new account without providing its seed.
    /// - If the account is already tracked and `overwrite` is set to `false`.
    /// - If `overwrite` is set to `true` and the `account_data` nonce is lower than the one already
    ///   being tracked.
    /// - If `overwrite` is set to `true` and the `account_data` hash doesn't match the network's
    ///   account hash.
    pub async fn add_account(
        &mut self,
        account: &Account,
        account_seed: Option<Word>,
        auth_secret_key: &AuthSecretKey,
        overwrite: bool,
    ) -> Result<(), ClientError> {
        let account_seed = if account.is_new() {
            if account_seed.is_none() {
                return Err(ClientError::AddNewAccountWithoutSeed);
            }
            account_seed
        } else {
            // Ignore the seed since it's not a new account

            // TODO: The alternative approach to this is to store the seed anyway, but
            // ignore it at the point of executing against this transaction, but that
            // approach seems a little bit more incorrect
            if account_seed.is_some() {
                tracing::warn!("Added an existing account and still provided a seed when it is not needed. It's possible that the account's file was incorrectly generated. The seed will be ignored.");
            }
            None
        };

        let tracked_account = self.store.get_account(account.id()).await?;

        match tracked_account {
            None => {
                // If the account is not being tracked, insert it into the store regardless of the
                // `overwrite` flag
                self.store.add_note_tag(account.try_into()?).await?;

                self.store
                    .insert_account(account, account_seed, auth_secret_key)
                    .await
                    .map_err(ClientError::StoreError)
            },
            Some(tracked_account) => {
                if !overwrite {
                    // Only overwrite the account if the flag is set to `true`
                    return Err(ClientError::AccountAlreadyTracked(account.id()));
                }

                if tracked_account.account().nonce().as_int() > account.nonce().as_int() {
                    // If the new account is older than the one being tracked, return an error
                    return Err(ClientError::AccountNonceTooLow);
                }

                if tracked_account.is_locked() {
                    // If the tracked account is locked, check that the account hash matches the one
                    // in the network
                    let network_account_hash =
                        self.rpc_api.get_account_update(account.id()).await?.hash();
                    if network_account_hash != account.hash() {
                        return Err(ClientError::AccountHashMismatch(network_account_hash));
                    }
                }

                self.store.update_account(account).await.map_err(ClientError::StoreError)
            },
        }
    }

    // ACCOUNT DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns a list of [AccountHeader] of all accounts stored in the database along with their
    /// statuses.
    ///
    /// Said accounts' state is the state after the last performed sync.
    pub async fn get_account_headers(
        &self,
    ) -> Result<Vec<(AccountHeader, AccountStatus)>, ClientError> {
        self.store.get_account_headers().await.map_err(|err| err.into())
    }

    /// Retrieves a full [AccountRecord] object for the specified `account_id`. This result
    /// represents data for the latest state known to the client, alongside its status. Returns
    /// `None` if the account ID is not found.
    pub async fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<AccountRecord>, ClientError> {
        self.store.get_account(account_id).await.map_err(|err| err.into())
    }

    /// Retrieves an [AccountHeader] object for the specified [AccountId] along with its status.
    /// Returns `None` if the account ID is not found.
    ///
    /// Said account's state is the state according to the last sync performed.
    pub async fn get_account_header_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<Option<(AccountHeader, AccountStatus)>, ClientError> {
        self.store.get_account_header(account_id).await.map_err(|err| err.into())
    }

    /// Returns an [AuthSecretKey] object utilized to authenticate an account. Returns `None`
    /// if the account ID is not found.
    pub async fn get_account_auth(
        &self,
        account_id: AccountId,
    ) -> Result<Option<AuthSecretKey>, ClientError> {
        self.store.get_account_auth(account_id).await.map_err(|err| err.into())
    }

    pub async fn get_account_or_error(
        &self,
        account_id: AccountId,
    ) -> Result<AccountRecord, ClientError> {
        self.get_account(account_id)
            .await?
            .ok_or(ClientError::AccountDataNotFound(account_id))
    }

    pub async fn get_account_header_or_error(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountHeader, AccountStatus), ClientError> {
        self.get_account_header_by_id(account_id)
            .await?
            .ok_or(ClientError::AccountDataNotFound(account_id))
    }

    pub async fn get_account_auth_or_error(
        &self,
        account_id: AccountId,
    ) -> Result<AuthSecretKey, ClientError> {
        self.get_account_auth(account_id)
            .await?
            .ok_or(ClientError::AccountDataNotFound(account_id))
    }
}

// ACCOUNT UPDATES
// ================================================================================================

/// Contains account changes to apply to the store.
pub struct AccountUpdates {
    /// Updated public accounts.
    updated_public_accounts: Vec<Account>,
    /// Node account hashes that don't match the current tracked state for private accounts.
    mismatched_private_accounts: Vec<(AccountId, Digest)>,
}

impl AccountUpdates {
    /// Creates a new instance of `AccountUpdates`.
    pub fn new(
        updated_public_accounts: Vec<Account>,
        mismatched_private_accounts: Vec<(AccountId, Digest)>,
    ) -> Self {
        Self {
            updated_public_accounts,
            mismatched_private_accounts,
        }
    }

    /// Returns the updated public accounts.
    pub fn updated_public_accounts(&self) -> &[Account] {
        &self.updated_public_accounts
    }

    /// Returns the mismatched private accounts.
    pub fn mismatched_private_accounts(&self) -> &[(AccountId, Digest)] {
        &self.mismatched_private_accounts
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use alloc::vec::Vec;

    use miden_lib::transaction::TransactionKernel;
    use miden_objects::{
        account::{Account, AccountData, AuthSecretKey},
        crypto::dsa::rpo_falcon512::SecretKey,
        testing::account_id::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        },
        Felt, Word,
    };

    use crate::mock::create_test_client;

    fn create_account_data(account_id: u128) -> AccountData {
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
    pub async fn try_add_account() {
        // generate test client
        let (mut client, _rpc_api) = create_test_client().await;

        let account = Account::mock(
            ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN,
            Felt::new(0),
            TransactionKernel::testing_assembler(),
        );

        let key_pair = SecretKey::new();

        assert!(client
            .add_account(&account, None, &AuthSecretKey::RpoFalcon512(key_pair.clone()), false)
            .await
            .is_err());
        assert!(client
            .add_account(
                &account,
                Some(Word::default()),
                &AuthSecretKey::RpoFalcon512(key_pair),
                false
            )
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn load_accounts_test() {
        // generate test client
        let (mut client, _) = create_test_client().await;

        let created_accounts_data = create_initial_accounts_data();

        for account_data in created_accounts_data.clone() {
            client
                .add_account(
                    &account_data.account,
                    account_data.account_seed,
                    &account_data.auth_secret_key,
                    false,
                )
                .await
                .unwrap();
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

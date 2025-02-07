//! The `account` module provides types and client APIs for managing accounts within the Miden
//! network.
//!
//! Accounts are foundational entities of the Miden protocol. They store assets and define
//! rules for manipulating them. Once an account is registered with the client, its state will
//! be updated accordingly, and validated against the network state on every sync.
//!
//! # Example
//!
//! To add a new account to the client's store, you might use the [`Client::add_account`] method as
//! follows:
//!
//! ```rust
//! # use miden_client::account::{Account, AccountBuilder, AccountType, component::BasicWallet};
//! # use miden_objects::account::{AuthSecretKey, AccountStorageMode};
//! # use miden_client::crypto::{FeltRng, SecretKey};
//! # async fn add_new_account_example(
//! #     client: &mut miden_client::Client<impl FeltRng>
//! # ) -> Result<(), miden_client::ClientError> {
//! #   let random_seed = Default::default();
//! let key_pair = SecretKey::with_rng(client.rng());
//!
//! let (account, seed) = AccountBuilder::new(random_seed)
//!     .account_type(AccountType::RegularAccountImmutableCode)
//!     .storage_mode(AccountStorageMode::Private)
//!     .with_component(BasicWallet)
//!     .build()?;
//!
//! // Add the account to the client. The account seed and authentication key are required
//! // for new accounts.
//! client.add_account(&account,
//!     Some(seed),
//!     &AuthSecretKey::RpoFalcon512(key_pair),
//!     false
//! ).await?;
//! #   Ok(())
//! # }
//! ```
//!
//! For more details on accounts, refer to the [Account] documentation.

use alloc::vec::Vec;

use miden_objects::{crypto::rand::FeltRng, Word};

use super::Client;
use crate::{
    store::{AccountRecord, AccountStatus},
    ClientError,
};

// RE-EXPORTS
// ================================================================================================
pub mod procedure_roots;

pub use miden_objects::account::{
    Account, AccountBuilder, AccountCode, AccountData, AccountHeader, AccountId, AccountStorage,
    AccountStorageMode, AccountType, StorageSlot,
};

pub mod component {
    pub use miden_lib::account::{
        auth::RpoFalcon512, faucets::BasicFungibleFaucet, wallets::BasicWallet,
    };
    pub use miden_objects::account::{
        AccountComponent, AccountComponentMetadata, AccountComponentTemplate, FeltRepresentation,
        InitStorageData, MapRepresentation, PlaceholderType, StorageEntry, StoragePlaceholder,
        StorageSlotType, StorageValue, WordRepresentation,
    };
}

// CLIENT METHODS
// ================================================================================================

/// This section of the [Client] contains methods for:
///
/// - **Account creation:** Use the [`AccountBuilder`] to construct new accounts, specifying account
///   type, storage mode (public/private), and attaching necessary components (e.g., basic wallet or
///   fungible faucet). After creation, they can be added to the client.
///
/// - **Account tracking:** Accounts added via the client are persisted to the local store, where
///   their state (including nonce, balance, and metadata) is updated upon every synchronization
///   with the network.
///
/// - **Data retrieval:** The module also provides methods to fetch account-related data.
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
    /// - If the account is new but no seed is provided.
    /// - If the account is already tracked and `overwrite` is set to `false`.
    /// - If `overwrite` is set to `true` and the `account_data` nonce is lower than the one already
    ///   being tracked.
    /// - If `overwrite` is set to `true` and the `account_data` hash doesn't match the network's
    ///   account hash.
    pub async fn add_account(
        &mut self,
        account: &Account,
        account_seed: Option<Word>,
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
                    .insert_account(account, account_seed)
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

    /// Attempts to retrieve an [AccountRecord] by its [AccountId].
    ///
    /// # Errors
    ///
    /// - If the account record is not found.
    /// - If the underlying store operation fails.
    pub async fn try_get_account(
        &self,
        account_id: AccountId,
    ) -> Result<AccountRecord, ClientError> {
        self.get_account(account_id)
            .await?
            .ok_or(ClientError::AccountDataNotFound(account_id))
    }

    /// Attempts to retrieve an [AccountHeader] by its [AccountId].
    ///
    /// # Errors
    ///
    /// - If the account header is not found.
    /// - If the underlying store operation fails.
    pub async fn try_get_account_header(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountHeader, AccountStatus), ClientError> {
        self.get_account_header_by_id(account_id)
            .await?
            .ok_or(ClientError::AccountDataNotFound(account_id))
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
        let (mut client, _rpc_api, _) = create_test_client().await;

        let account = Account::mock(
            ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN,
            Felt::new(0),
            TransactionKernel::testing_assembler(),
        );

        assert!(client.add_account(&account, None, false).await.is_err());
        assert!(client.add_account(&account, Some(Word::default()), false).await.is_ok());
    }

    #[tokio::test]
    async fn load_accounts_test() {
        // generate test client
        let (mut client, ..) = create_test_client().await;

        let created_accounts_data = create_initial_accounts_data();

        for account_data in created_accounts_data.clone() {
            client
                .add_account(&account_data.account, account_data.account_seed, false)
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

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
//! # use miden_client::{
//! #   account::{Account, AccountBuilder, AccountType, component::BasicWallet},
//! #   crypto::FeltRng
//! # };
//! # use miden_objects::account::AccountStorageMode;
//! # async fn add_new_account_example(
//! #     client: &mut miden_client::Client
//! # ) -> Result<(), miden_client::ClientError> {
//! #   let random_seed = Default::default();
//! let (account, seed) = AccountBuilder::new(random_seed)
//!     .account_type(AccountType::RegularAccountImmutableCode)
//!     .storage_mode(AccountStorageMode::Private)
//!     .with_component(BasicWallet)
//!     .build()?;
//!
//! // Add the account to the client. The account seed and authentication key are required
//! // for new accounts.
//! client.add_account(&account, Some(seed), false).await?;
//! #   Ok(())
//! # }
//! ```
//!
//! For more details on accounts, refer to the [Account] documentation.

use alloc::{string::ToString, vec::Vec};

use miden_lib::account::{auth::RpoFalcon512, wallets::BasicWallet};
use miden_objects::{
    AccountError, Word, block::BlockHeader, crypto::dsa::rpo_falcon512::PublicKey,
};

use super::Client;
use crate::{
    errors::ClientError,
    rpc::domain::account::AccountDetails,
    store::{AccountRecord, AccountStatus},
};

pub mod procedure_roots;

// RE-EXPORTS
// ================================================================================================

pub use miden_objects::account::{
    Account, AccountBuilder, AccountCode, AccountDelta, AccountFile, AccountHeader, AccountId,
    AccountStorage, AccountStorageMode, AccountType, StorageMap, StorageSlot,
};

pub mod component {
    pub const COMPONENT_TEMPLATE_EXTENSION: &str = "mct";

    pub use miden_lib::account::{
        auth::RpoFalcon512, faucets::BasicFungibleFaucet, wallets::BasicWallet,
    };
    pub use miden_objects::account::{
        AccountComponent, AccountComponentMetadata, AccountComponentTemplate, FeltRepresentation,
        InitStorageData, StorageEntry, StorageSlotType, StorageValueName, TemplateType,
        WordRepresentation,
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
impl Client {
    // ACCOUNT CREATION
    // --------------------------------------------------------------------------------------------

    /// Adds the provided [Account] in the store so it can start being tracked by the client.
    ///
    /// If the account is already being tracked and `overwrite` is set to `true`, the account will
    /// be overwritten. The `account_seed` should be provided if the account is newly created.
    ///
    /// # Errors
    ///
    /// - If the account is new but no seed is provided.
    /// - If the account is already tracked and `overwrite` is set to `false`.
    /// - If `overwrite` is set to `true` and the `account_data` nonce is lower than the one already
    ///   being tracked.
    /// - If `overwrite` is set to `true` and the `account_data` commitment doesn't match the
    ///   network's account commitment.
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
                tracing::warn!(
                    "Added an existing account and still provided a seed when it is not needed. It's possible that the account's file was incorrectly generated. The seed will be ignored."
                );
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
                    // If the tracked account is locked, check that the account commitment matches
                    // the one in the network
                    let network_account_commitment =
                        self.rpc_api.get_account_details(account.id()).await?.commitment();
                    if network_account_commitment != account.commitment() {
                        return Err(ClientError::AccountCommitmentMismatch(
                            network_account_commitment,
                        ));
                    }
                }

                self.store.update_account(account).await.map_err(ClientError::StoreError)
            },
        }
    }

    /// Imports an account from the network to the client's store. The account needs to be public
    /// and be tracked by the network, it will be fetched by its ID. If the account was already
    /// being tracked by the client, it's state will be overwritten.
    ///
    /// # Errors
    /// - If the account is not found on the network.
    /// - If the account is private.
    /// - There was an error sending the request to the network.
    pub async fn import_account_by_id(&mut self, account_id: AccountId) -> Result<(), ClientError> {
        let account_details = self.rpc_api.get_account_details(account_id).await?;

        let account = match account_details {
            AccountDetails::Private(..) => {
                return Err(ClientError::AccountIsPrivate(account_id));
            },
            AccountDetails::Public(account, ..) => account,
        };

        self.add_account(&account, None, true).await
    }

    // ACCOUNT DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns a list of [`AccountHeader`] of all accounts stored in the database along with their
    /// statuses.
    ///
    /// Said accounts' state is the state after the last performed sync.
    pub async fn get_account_headers(
        &self,
    ) -> Result<Vec<(AccountHeader, AccountStatus)>, ClientError> {
        self.store.get_account_headers().await.map_err(Into::into)
    }

    /// Retrieves a full [`AccountRecord`] object for the specified `account_id`. This result
    /// represents data for the latest state known to the client, alongside its status. Returns
    /// `None` if the account ID is not found.
    pub async fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<AccountRecord>, ClientError> {
        self.store.get_account(account_id).await.map_err(Into::into)
    }

    /// Retrieves an [`AccountHeader`] object for the specified [`AccountId`] along with its status.
    /// Returns `None` if the account ID is not found.
    ///
    /// Said account's state is the state according to the last sync performed.
    pub async fn get_account_header_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<Option<(AccountHeader, AccountStatus)>, ClientError> {
        self.store.get_account_header(account_id).await.map_err(Into::into)
    }

    /// Attempts to retrieve an [`AccountRecord`] by its [`AccountId`].
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

    /// Attempts to retrieve an [`AccountHeader`] by its [`AccountId`].
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

// UTILITY FUNCTIONS
// ================================================================================================

/// Builds an regular account ID from the provided parameters. The ID may be used along
/// `Client::import_account_by_id` to import a public account from the network (provided that the
/// used seed is known).
///
/// This function will only work for accounts with the [`BasicWallet`] and [`RpoFalcon512`]
/// components.
///
/// # Arguments
/// - `init_seed`: Initial seed used to create the account. This is the seed passed to
///   [`AccountBuilder::new`].
/// - `public_key`: Public key of the account used in the [`RpoFalcon512`] component.
/// - `storage_mode`: Storage mode of the account.
/// - `is_mutable`: Whether the account is mutable or not.
/// - `anchor_block`: Anchor block of the account.
///
/// # Errors
/// - If the provided block header is not an anchor block.
/// - If the account cannot be built.
pub fn build_wallet_id(
    init_seed: [u8; 32],
    public_key: PublicKey,
    storage_mode: AccountStorageMode,
    is_mutable: bool,
    anchor_block: &BlockHeader,
) -> Result<AccountId, ClientError> {
    let account_type = if is_mutable {
        AccountType::RegularAccountUpdatableCode
    } else {
        AccountType::RegularAccountImmutableCode
    };

    let accound_id_anchor = anchor_block.try_into().map_err(|_| {
        ClientError::AccountError(AccountError::AssumptionViolated(
            "Provided block header is not an anchor block".to_string(),
        ))
    })?;

    let (account, _) = AccountBuilder::new(init_seed)
        .anchor(accound_id_anchor)
        .account_type(account_type)
        .storage_mode(storage_mode)
        .with_component(RpoFalcon512::new(public_key))
        .with_component(BasicWallet)
        .build()?;

    Ok(account.id())
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use alloc::vec::Vec;

    use miden_lib::transaction::TransactionKernel;
    use miden_objects::{
        Felt, Word,
        account::{Account, AccountFile, AuthSecretKey},
        crypto::dsa::rpo_falcon512::SecretKey,
        testing::account_id::{
            ACCOUNT_ID_PRIVATE_FUNGIBLE_FAUCET, ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET,
        },
    };

    use crate::tests::create_test_client;

    fn create_account_data(account_id: u128) -> AccountFile {
        let account =
            Account::mock(account_id, Felt::new(2), TransactionKernel::testing_assembler());

        AccountFile::new(
            account.clone(),
            Some(Word::default()),
            AuthSecretKey::RpoFalcon512(SecretKey::new()),
        )
    }

    pub fn create_initial_accounts_data() -> Vec<AccountFile> {
        let account = create_account_data(ACCOUNT_ID_PRIVATE_FUNGIBLE_FAUCET);

        let faucet_account = create_account_data(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET);

        // Create Genesis state and save it to a file
        let accounts = vec![account, faucet_account];

        accounts
    }

    #[tokio::test]
    pub async fn try_add_account() {
        // generate test client
        let (mut client, _rpc_api, _) = create_test_client().await;

        let account = Account::mock(
            ACCOUNT_ID_PRIVATE_FUNGIBLE_FAUCET,
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
            assert_eq!(client_acc.0.commitment(), expected_acc.commitment());
        }
    }
}

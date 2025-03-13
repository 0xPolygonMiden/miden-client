//! The `authenticator` module provides the [`ClientAuthenticator`] type, which is used to sign
//! transactions with the account's secret key. The authenticator is based on a [`KeyStore`] that
//! stores and manages the account's secret keys.
//!
//! The [`KeyStore`] trait defines the interface for storing and retrieving secret keys. The
//! `FilesystemKeyStore` and `WebKeyStore` types (for std and no-std respectively) are provided
//! as implementations of the trait. The keystore is used to retrieve the secret key for a given
//! public key when signing transactions.
//!
//! //! # Example
//!
//! A reference to the keystore should be kept so that new keys can be added to it when new accounts
//! are created. You might use the [`KeyStore::add_key`] method as follows:
//!
//! ```rust
//! # use miden_client::{
//! #    account::{Account, AccountBuilder, AccountType, component::RpoFalcon512},
//! #    authenticator::keystore::{KeyStore, FilesystemKeyStore},
//! #    crypto::{FeltRng, SecretKey}
//! # };
//! # use miden_objects::account::{AuthSecretKey, AccountStorageMode};
//! # async fn add_new_account_example(
//! #     client: &mut miden_client::Client<impl FeltRng>,
//! #     keystore: &mut FilesystemKeyStore,
//! # ) {
//! #   let random_seed = Default::default();
//! let key_pair = SecretKey::with_rng(client.rng());
//!
//! let (account, seed) = AccountBuilder::new(random_seed)
//!     .with_component(RpoFalcon512::new(key_pair.public_key()))
//!     .build()
//!     .unwrap();
//!
//! // Add the secret key to the keystore so the account can sign transactions
//! keystore.add_key(&AuthSecretKey::RpoFalcon512(key_pair)).await.unwrap();
//!
//! // Add the account to the client. The account seed and authentication key are required
//! // for new accounts.
//! client.add_account(&account, Some(seed), false).await.unwrap();
//! # }
//! ```
use alloc::{string::ToString, sync::Arc, vec::Vec};

use miden_objects::{
    Digest, Felt, Word,
    account::{AccountDelta, AuthSecretKey},
};
use miden_tx::{AuthenticationError, auth::TransactionAuthenticator, utils::sync::RwLock};
use rand::Rng;

use crate::authenticator::keystore::KeyStore;

pub mod keystore;

/// An account authenticator based on a [`KeyStore`].
#[derive(Debug, Clone)]
pub struct ClientAuthenticator<R, K> {
    /// The random number generator used to generate signatures.
    rng: Arc<RwLock<R>>,
    /// The key store used to retrieve secret keys.
    keystore: K,
}

impl<R: Rng, K: KeyStore> ClientAuthenticator<R, K> {
    /// Creates a new instance of the authenticator.
    pub fn new(rng: R, keystore: K) -> Self {
        ClientAuthenticator {
            rng: Arc::new(RwLock::new(rng)),
            keystore,
        }
    }
}

impl<R: Rng, K: KeyStore> TransactionAuthenticator for ClientAuthenticator<R, K> {
    /// Gets a signature over a message, given a public key.
    ///
    /// The pub key should correspond to one of the keys tracked by the authenticator's store.
    ///
    /// # Errors
    /// If the public key isn't found in the store, [`AuthenticationError::UnknownPublicKey`] is
    /// returned.
    fn get_signature(
        &self,
        pub_key: Word,
        message: Word,
        _account_delta: &AccountDelta,
    ) -> Result<Vec<Felt>, AuthenticationError> {
        let mut rng = self.rng.write();

        let secret_key = self
            .keystore
            .get_key(pub_key)
            .map_err(|err| AuthenticationError::other(err.to_string()))?;

        let AuthSecretKey::RpoFalcon512(k) = secret_key
            .ok_or(AuthenticationError::UnknownPublicKey(Digest::from(pub_key).into()))?;

        miden_tx::auth::signatures::get_falcon_signature(&k, message, &mut *rng)
    }
}

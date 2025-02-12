use alloc::{string::ToString, sync::Arc, vec::Vec};

use miden_objects::{
    account::{AccountDelta, AuthSecretKey},
    Digest, Felt, Word,
};
use miden_tx::{auth::TransactionAuthenticator, utils::sync::RwLock, AuthenticationError};
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

use alloc::{sync::Arc, vec::Vec};

use miden_objects::{
    accounts::{AccountDelta, AuthSecretKey},
    Digest, Felt, Word,
};
use miden_tx::{auth::TransactionAuthenticator, utils::sync::RwLock, AuthenticationError};
use pollster::FutureExt as _;
use rand::Rng;

use super::Store;

/// Represents an authenticator based on a [Store].
pub struct StoreAuthenticator<R> {
    store: Arc<dyn Store>,
    rng: Arc<RwLock<R>>,
}

impl<R: Rng> StoreAuthenticator<R> {
    pub fn new_with_rng(store: Arc<dyn Store>, rng: R) -> Self {
        StoreAuthenticator { store, rng: Arc::new(RwLock::new(rng)) }
    }
}

impl<R: Rng> TransactionAuthenticator for StoreAuthenticator<R> {
    /// Gets a signature over a message, given a public key.
    ///
    /// The pub key should correspond to one of the keys tracked by the authenticator's store.
    ///
    /// # Errors
    /// If the public key isn't found in the store, [AuthenticationError::UnknownPublicKey] is
    /// returned.
    fn get_signature(
        &self,
        pub_key: Word,
        message: Word,
        _account_delta: &AccountDelta,
    ) -> Result<Vec<Felt>, AuthenticationError> {
        let mut rng = self.rng.write();

        let secret_key =
            self.store.get_account_auth_by_pub_key(pub_key).block_on().map_err(|err| {
                AuthenticationError::other_with_source("error getting secret key from Store", err)
            })?;

        let AuthSecretKey::RpoFalcon512(k) = secret_key
            .ok_or(AuthenticationError::UnknownPublicKey(Digest::from(pub_key).into()))?;
        miden_tx::auth::signatures::get_falcon_signature(&k, message, &mut *rng)
    }
}

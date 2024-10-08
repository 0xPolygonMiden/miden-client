use alloc::{sync::Arc, vec::Vec};
use core::cell::RefCell;

use miden_objects::{
    accounts::{AccountDelta, AuthSecretKey},
    Digest, Felt, Word,
};
use miden_tx::{auth::TransactionAuthenticator, AuthenticationError};
use rand::Rng;

use super::Store;

/// Represents an authenticator based on a [Store]
pub struct StoreAuthenticator<R> {
    store: Arc<dyn Store>,
    rng: RefCell<R>,
}

impl<R: Rng> StoreAuthenticator<R> {
    pub fn new_with_rng(store: Arc<dyn Store>, rng: R) -> Self {
        StoreAuthenticator { store, rng: RefCell::new(rng) }
    }
}

impl<R: Rng> TransactionAuthenticator for StoreAuthenticator<R> {
    /// Gets a signature over a message, given a public key.
    ///
    /// The pub key should correspond to one of the keys tracked by the authenticator's store.
    ///
    /// # Errors
    /// If the public key is not found in the store, [AuthenticationError::UnknownKey] is
    /// returned.
    fn get_signature(
        &self,
        pub_key: Word,
        message: Word,
        _account_delta: &AccountDelta,
    ) -> Result<Vec<Felt>, AuthenticationError> {
        let mut rng = self.rng.borrow_mut();

        let secret_key = self
            .store
            .get_account_auth_by_pub_key(pub_key)
            .map_err(|_| AuthenticationError::UnknownKey(format!("{}", Digest::from(pub_key))))?;

        let AuthSecretKey::RpoFalcon512(k) = secret_key;
        miden_tx::auth::signatures::get_falcon_signature(&k, message, &mut *rng)
    }
}

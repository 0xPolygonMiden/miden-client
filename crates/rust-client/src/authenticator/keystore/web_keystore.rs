use alloc::string::ToString;
use alloc::sync::Arc;
use miden_objects::{
    account::{AccountDelta, AuthSecretKey},
    Digest, Felt, Word,
};
use miden_tx::{auth::TransactionAuthenticator, AuthenticationError};
use rand::Rng;
use miden_tx::utils::sync::RwLock;
use super::{KeyStore, KeyStoreError};
use crate::store::web_store::account::utils::{get_account_auth_by_pub_key, insert_account_auth};

/// A web-based keystore that stores keys in [browser's local storage](https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API)
/// and provides transaction authentication functionality.
#[derive(Clone)]
pub struct WebKeyStore<R: Rng> {
    /// The random number generator used to generate signatures.
    rng: Arc<RwLock<R>>,
}

impl<R: Rng> WebKeyStore<R> {
    /// Creates a new instance of the web keystore with the provided RNG.
    pub fn new(rng: R) -> Self {
        WebKeyStore {
            rng: Arc::new(RwLock::new(rng)),
        }
    }
}

impl<R: Rng> KeyStore for WebKeyStore<R> {
    fn add_key(&self, key: &AuthSecretKey) -> Result<(), KeyStoreError> {
        let window = web_sys::window()
            .ok_or_else(|| KeyStoreError::StorageError("Window not available".to_string()))?;
        let pub_key = match &key {
            AuthSecretKey::RpoFalcon512(k) => Digest::from(Word::from(k.public_key())).to_hex(),
        };
        let secret_key_hex = hex::encode(key.to_bytes());

        insert_account_auth(pub_key, secret_key_hex).await.map_err(|_| {
            KeyStoreError::StorageError("Failed to insert item into local storage".to_string())
        })?;

        Ok(())
    }

    fn get_key(&self, pub_key: Word) -> Result<Option<AuthSecretKey>, KeyStoreError> {
        let pub_key_str = Digest::from(pub_key).to_hex();
        let secret_key_hex = get_account_auth_by_pub_key(pub_key_str).map_err(|_| {
            KeyStoreError::StorageError("Failed to get item from local storage".to_string())
        })?;

        let secret_key_bytes = hex::decode(secret_key_hex).map_err(|err| {
            KeyStoreError::DecodingError(format!("error decoding secret key hex: {err:?}"))
        })?;

        let secret_key = AuthSecretKey::read_from_bytes(&secret_key_bytes).map_err(|err| {
            KeyStoreError::DecodingError(format!("error reading secret key: {err:?}"))
        })?;

        Ok(Some(secret_key))
    }
}

impl<R: Rng> TransactionAuthenticator for WebKeyStore<R> {
    /// Gets a signature over a message, given a public key.
    ///
    /// The public key should correspond to one of the keys tracked by the keystore.
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
        let secret_key = self.get_key(pub_key).map_err(|err| AuthenticationError::other(err.to_string()))?;
        let AuthSecretKey::RpoFalcon512(k) = secret_key
            .ok_or(AuthenticationError::UnknownPublicKey(Digest::from(pub_key).into()))?;
        miden_tx::auth::signatures::get_falcon_signature(&k, message, &mut *rng)
    }
}

impl<R: Rng> TransactionAuthenticator for WebKeyStore<R> {
    /// Gets a signature over a message, given a public key.
    ///
    /// The public key should correspond to one of the keys tracked by the keystore.
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
        let secret_key = self.get_key(pub_key).map_err(|err| AuthenticationError::other(err.to_string()))?;
        let AuthSecretKey::RpoFalcon512(k) = secret_key
            .ok_or(AuthenticationError::UnknownPublicKey(Digest::from(pub_key).into()))?;
        miden_tx::auth::signatures::get_falcon_signature(&k, message, &mut *rng)
    }
}

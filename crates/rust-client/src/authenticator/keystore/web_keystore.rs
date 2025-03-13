use alloc::{boxed::Box, string::ToString};

use miden_objects::{
    Digest, Word,
    account::AuthSecretKey,
    utils::{Deserializable, Serializable},
};
use tonic::async_trait;

use super::{KeyStore, KeyStoreError};
use crate::store::web_store::account::utils::{get_account_auth_by_pub_key, insert_account_auth};

/// A web-based keystore that stores keys in [browser's local storage](https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API).
#[derive(Clone)]
pub struct WebKeyStore;

#[async_trait(?Send)]
impl KeyStore for WebKeyStore {
    async fn add_key(&self, key: &AuthSecretKey) -> Result<(), KeyStoreError> {
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

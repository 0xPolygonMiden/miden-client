use alloc::string::ToString;

use miden_objects::{
    Digest, Word,
    account::AuthSecretKey,
    utils::{Deserializable, Serializable},
};

use super::{KeyStore, KeyStoreError};

/// A web-based keystore that stores keys in [browser's local storage](https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API).
#[derive(Debug, Clone)]
pub struct WebKeyStore;

impl KeyStore for WebKeyStore {
    fn add_key(&self, key: &AuthSecretKey) -> Result<(), KeyStoreError> {
        let window = web_sys::window()
            .ok_or_else(|| KeyStoreError::StorageError("Window not available".to_string()))?;
        let pub_key = match &key {
            AuthSecretKey::RpoFalcon512(k) => Digest::from(Word::from(k.public_key())).to_hex(),
        };

        let secret_key_hex = hex::encode(key.to_bytes());

        let storage = window
            .local_storage()
            .map_err(|_| KeyStoreError::StorageError("Local storage not available".to_string()))?
            .ok_or_else(|| {
                KeyStoreError::StorageError("Local storage not available".to_string())
            })?;

        storage.set_item(&pub_key, &secret_key_hex).map_err(|_| {
            KeyStoreError::StorageError("Failed to set item in local storage".to_string())
        })?;

        Ok(())
    }

    fn get_key(&self, pub_key: Word) -> Result<Option<AuthSecretKey>, KeyStoreError> {
        let window = web_sys::window()
            .ok_or_else(|| KeyStoreError::StorageError("Window not available".to_string()))?;
        let pub_key_str = Digest::from(pub_key).to_hex();

        let storage = window
            .local_storage()
            .map_err(|_| KeyStoreError::StorageError("Local storage not available".to_string()))?
            .ok_or_else(|| {
                KeyStoreError::StorageError("Local storage not available".to_string())
            })?;
        let secret_key_hex = storage.get_item(&pub_key_str).map_err(|_| {
            KeyStoreError::StorageError("Failed to get item from local storage".to_string())
        })?;

        match secret_key_hex {
            Some(secret_key_hex) => {
                let secret_key_bytes = hex::decode(secret_key_hex).map_err(|err| {
                    KeyStoreError::DecodingError(format!("error decoding secret key hex: {err:?}"))
                })?;

                let secret_key = AuthSecretKey::read_from_bytes(secret_key_bytes.as_slice())
                    .map_err(|err| {
                        KeyStoreError::DecodingError(format!("error reading secret key: {err:?}"))
                    })?;

                Ok(Some(secret_key))
            },
            None => Ok(None),
        }
    }
}

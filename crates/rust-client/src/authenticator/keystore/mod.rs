use alloc::string::String;

use miden_objects::{account::AuthSecretKey, Word};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyStoreError {
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Decoding error: {0}")]
    DecodingError(String),
}

pub trait KeyStore {
    /// Adds a new key to the keystore. If a key with the same public key already exists, it
    /// will be overwritten.
    fn add_key(&self, key: &AuthSecretKey) -> Result<(), KeyStoreError>;

    /// Gets a secret key by public key. If the public key isn't found, `None` is returned.
    fn get_key(&self, pub_key: Word) -> Result<Option<AuthSecretKey>, KeyStoreError>;
}

#[cfg(target_arch = "wasm32")]
mod web_keystore;
#[cfg(target_arch = "wasm32")]
pub use web_keystore::WebKeyStore;

#[cfg(not(target_arch = "wasm32"))]
mod fs_keystore;
#[cfg(not(target_arch = "wasm32"))]
pub use fs_keystore::FilesystemKeyStore;

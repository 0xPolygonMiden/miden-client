use alloc::string::String;
use core::fmt::Debug;

use miden_objects::{Word, account::AuthSecretKey};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyStoreError {
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Decoding error: {0}")]
    DecodingError(String),
}

pub trait KeyStore: Debug + Send + Sync {
    /// Adds a new key to the keystore. If a key with the same public key already exists, it
    /// will be overwritten.
    fn add_key(&self, key: &AuthSecretKey) -> Result<(), KeyStoreError>;

    /// Gets a secret key by public key. If the public key isn't found, `None` is returned.
    fn get_key(&self, pub_key: Word) -> Result<Option<AuthSecretKey>, KeyStoreError>;
}

#[cfg(feature = "std")]
mod fs_keystore;
#[cfg(feature = "std")]
pub use fs_keystore::FilesystemKeyStore;

#[cfg(not(feature = "std"))]
mod web_keystore;
#[cfg(not(feature = "std"))]
pub use web_keystore::WebKeyStore;

use alloc::{boxed::Box, string::String};
use std::{
    fs::OpenOptions,
    io::{BufReader, Read},
    path::PathBuf,
};

use miden_objects::{Digest, Word, account::AuthSecretKey};
use miden_tx::utils::{Deserializable, Serializable};
use tokio::io::AsyncWriteExt;
use tonic::async_trait;

use super::{KeyStore, KeyStoreError};

/// A filesystem-based keystore that stores keys in separate files. The public key is used as the
/// filename and the contents of the file are the serialized secret key.
#[derive(Clone)]
pub struct FilesystemKeyStore {
    /// The directory where the keys are stored.
    keys_directory: PathBuf,
}

impl FilesystemKeyStore {
    /// Creates a new filesystem keystore with the given keys directory. If the directory doesn't
    /// exist, it will be created.
    pub fn new(keys_directory: PathBuf) -> Result<Self, KeyStoreError> {
        if !keys_directory.exists() {
            std::fs::create_dir_all(&keys_directory).map_err(|err| {
                KeyStoreError::StorageError(format!("error creating keys directory: {err:?}"))
            })?;
        }

        Ok(FilesystemKeyStore { keys_directory })
    }
}

#[async_trait(?Send)]
impl KeyStore for FilesystemKeyStore {
    /// Adds a new key to the keystore. If a key with the same public key already exists, it
    /// will be overwritten.
    async fn add_key(&self, key: &AuthSecretKey) -> Result<(), KeyStoreError> {
        let pub_key = match &key {
            AuthSecretKey::RpoFalcon512(k) => Digest::from(Word::from(k.public_key())).to_hex(),
        };

        let file_path = self.keys_directory.join(&pub_key);

        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&file_path)
            .await
            .map_err(|err| {
                KeyStoreError::StorageError(format!("error opening secret key file: {err:?}"))
            })?;

        let mut writer = tokio::io::BufWriter::new(file);
        let mut key_pair_hex = hex::encode(key.to_bytes());

        // Ensure a newline is added so it is read correctly later
        key_pair_hex.push('\n');

        writer.write_all(key_pair_hex.as_bytes()).await.map_err(|err| {
            KeyStoreError::StorageError(format!("error writing secret key file: {err:?}"))
        })?;

        // Ensure the data is written before closing the file
        writer.flush().await.map_err(|err| {
            KeyStoreError::StorageError(format!("error flushing secret key file: {err:?}"))
        })?;

        Ok(())
    }

    /// Retrieves a secret key by its public key. Returns `None` if the key is not found.
    fn get_key(&self, pub_key: Word) -> Result<Option<AuthSecretKey>, KeyStoreError> {
        let pub_key_str = Digest::from(pub_key).to_hex();
        let file_path = self.keys_directory.join(pub_key_str);

        if !file_path.exists() {
            return Ok(None);
        }

        let file = OpenOptions::new().read(true).open(&file_path).map_err(|err| {
            KeyStoreError::StorageError(format!("error opening secret key file: {err:?}"))
        })?;

        let mut reader = BufReader::new(file);
        let mut key_pair_hex = String::new();

        // Read the entire file contents instead of just one line
        reader.read_to_string(&mut key_pair_hex).map_err(|err| {
            KeyStoreError::StorageError(format!("error reading secret key file: {err:?}"))
        })?;

        let secret_key_bytes = hex::decode(key_pair_hex.trim()).map_err(|err| {
            KeyStoreError::DecodingError(format!("error decoding secret key hex: {err:?}"))
        })?;

        let secret_key =
            AuthSecretKey::read_from_bytes(secret_key_bytes.as_slice()).map_err(|err| {
                KeyStoreError::DecodingError(format!(
                    "error reading secret key from bytes: {err:?}"
                ))
            })?;

        Ok(Some(secret_key))
    }
}

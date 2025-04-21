use alloc::string::String;
use std::{
    fs::OpenOptions,
    hash::{DefaultHasher, Hash, Hasher},
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    string::ToString,
    sync::Arc,
    vec::Vec,
};

use miden_objects::{
    Digest, Felt, Word,
    account::{AccountDelta, AuthSecretKey},
};
use miden_tx::{
    AuthenticationError,
    auth::TransactionAuthenticator,
    utils::{Deserializable, Serializable, sync::RwLock},
};
use rand::{Rng, SeedableRng};

use super::KeyStoreError;

/// A filesystem-based keystore that stores keys in separate files and provides transaction
/// authentication functionality. The public key is hashed and the result is used as the filename
/// and the contents of the file are the serialized public and secret key.
///
/// The keystore requires an RNG component for generating Falcon signatures at the moment of
/// transaction signing.
#[derive(Debug, Clone)]
pub struct FilesystemKeyStore<R: Rng> {
    /// The random number generator used to generate signatures.
    rng: Arc<RwLock<R>>,
    /// The directory where the keys are stored and read from.
    keys_directory: PathBuf,
}

impl<R: Rng> FilesystemKeyStore<R> {
    pub fn with_rng(keys_directory: PathBuf, rng: R) -> Result<Self, KeyStoreError> {
        if !keys_directory.exists() {
            std::fs::create_dir_all(&keys_directory).map_err(|err| {
                KeyStoreError::StorageError(format!("error creating keys directory: {err:?}"))
            })?;
        }

        Ok(FilesystemKeyStore {
            keys_directory,
            rng: Arc::new(RwLock::new(rng)),
        })
    }

    /// Adds a secret key to the keystore.
    pub fn add_key(&self, key: &AuthSecretKey) -> Result<(), KeyStoreError> {
        let pub_key = match key {
            AuthSecretKey::RpoFalcon512(k) => Word::from(k.public_key()),
        };

        let filename = hash_pub_key(pub_key);

        let file_path = self.keys_directory.join(filename);
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path)
            .map_err(|err| {
                KeyStoreError::StorageError(format!("error opening secret key file: {err:?}"))
            })?;

        let mut writer = BufWriter::new(file);
        let key_pair_hex = hex::encode(key.to_bytes());
        writer.write_all(key_pair_hex.as_bytes()).map_err(|err| {
            KeyStoreError::StorageError(format!("error writing secret key file: {err:?}"))
        })?;

        Ok(())
    }

    /// Retrieves a secret key from the keystore given its public key.
    pub fn get_key(&self, pub_key: Word) -> Result<Option<AuthSecretKey>, KeyStoreError> {
        let filename = hash_pub_key(pub_key);

        let file_path = self.keys_directory.join(filename);
        if !file_path.exists() {
            return Ok(None);
        }

        let file = OpenOptions::new().read(true).open(file_path).map_err(|err| {
            KeyStoreError::StorageError(format!("error opening secret key file: {err:?}"))
        })?;
        let mut reader = BufReader::new(file);
        let mut key_pair_hex = String::new();
        reader.read_line(&mut key_pair_hex).map_err(|err| {
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

// Provide a default implementation for `StdRng` so you can call FilesystemKeyStore::new() without
// type annotations.
impl FilesystemKeyStore<rand::rngs::StdRng> {
    /// Creates a new [`FilesystemKeyStore`] using [`rand::rngs::StdRng`] as the RNG.
    pub fn new(keys_directory: PathBuf) -> Result<Self, KeyStoreError> {
        use rand::rngs::StdRng;
        let rng = StdRng::from_os_rng();

        FilesystemKeyStore::with_rng(keys_directory, rng)
    }
}

impl<R: Rng> TransactionAuthenticator for FilesystemKeyStore<R> {
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

        let secret_key = self
            .get_key(pub_key)
            .map_err(|err| AuthenticationError::other(err.to_string()))?;

        let AuthSecretKey::RpoFalcon512(k) = secret_key
            .ok_or(AuthenticationError::UnknownPublicKey(Digest::from(pub_key).into()))?;

        miden_tx::auth::signatures::get_falcon_signature(&k, message, &mut *rng)
    }
}

/// Hashes a public key to a string representation.
fn hash_pub_key(pub_key: Word) -> String {
    let pub_key = Digest::from(pub_key).to_hex();
    let mut hasher = DefaultHasher::new();
    pub_key.hash(&mut hasher);
    hasher.finish().to_string()
}

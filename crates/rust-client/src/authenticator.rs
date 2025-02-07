use alloc::{string::String, sync::Arc, vec::Vec};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
};

use miden_objects::{
    account::{AccountDelta, AuthSecretKey},
    Digest, Felt, Word,
};
pub use miden_tx::AuthenticationError;
use miden_tx::{
    auth::TransactionAuthenticator,
    utils::{sync::RwLock, Deserializable, Serializable},
};
use rand::Rng;

#[derive(Debug, Clone)]
pub struct ClientAuthenticator<R> {
    keys_directory: PathBuf,
    rng: Arc<RwLock<R>>,
}

impl<R: Rng> ClientAuthenticator<R> {
    pub fn new_with_rng(keys_directory: PathBuf, rng: R) -> Result<Self, AuthenticationError> {
        if !keys_directory.exists() {
            std::fs::create_dir_all(&keys_directory).map_err(|err| {
                AuthenticationError::other_with_source("error creating keys directory", err)
            })?;
        }

        Ok(ClientAuthenticator {
            keys_directory,
            rng: Arc::new(RwLock::new(rng)),
        })
    }

    pub fn add_key(&self, key: AuthSecretKey) -> Result<(), AuthenticationError> {
        let pub_key = match &key {
            AuthSecretKey::RpoFalcon512(k) => Digest::from(Word::from(k.public_key())).to_hex(),
        };

        let file_path = self.keys_directory.join(pub_key);
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path)
            .map_err(|err| {
                AuthenticationError::other_with_source("error opening secret key file", err)
            })?;

        let mut writer = BufWriter::new(file);
        let key_pair_hex = hex::encode(key.to_bytes());
        writer.write_all(key_pair_hex.as_bytes()).map_err(|err| {
            AuthenticationError::other_with_source("error writing secret key file", err)
        })?;

        Ok(())
    }

    pub fn get_auth_by_pub_key(
        &self,
        pub_key: Word,
    ) -> Result<Option<AuthSecretKey>, AuthenticationError> {
        let pub_key_str = Digest::from(pub_key).to_hex();

        let file_path = self.keys_directory.join(pub_key_str);
        if !file_path.exists() {
            return Ok(None);
        }

        let file = OpenOptions::new().read(true).open(file_path).map_err(|err| {
            AuthenticationError::other_with_source("error opening secret key file", err)
        })?;
        let mut reader = BufReader::new(file);
        let mut key_pair_hex = String::new();
        reader.read_line(&mut key_pair_hex).map_err(|err| {
            AuthenticationError::other_with_source("error reading secret key file", err)
        })?;

        let secret_key_bytes = hex::decode(key_pair_hex.trim()).map_err(|err| {
            AuthenticationError::other_with_source("error decoding secret key hex", err)
        })?;
        let secret_key =
            AuthSecretKey::read_from_bytes(secret_key_bytes.as_slice()).map_err(|err| {
                AuthenticationError::other_with_source("error reading secret key from bytes", err)
            })?;

        Ok(Some(secret_key))
    }
}

impl<R: Rng> TransactionAuthenticator for ClientAuthenticator<R> {
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

        let secret_key = self.get_auth_by_pub_key(pub_key)?;

        let AuthSecretKey::RpoFalcon512(k) = secret_key
            .ok_or(AuthenticationError::UnknownPublicKey(Digest::from(pub_key).into()))?;

        miden_tx::auth::signatures::get_falcon_signature(&k, message, &mut *rng)
    }
}

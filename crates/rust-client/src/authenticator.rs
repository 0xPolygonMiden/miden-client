use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
};

use miden_objects::{
    account::{AccountDelta, AuthSecretKey},
    Digest, Felt, Word,
};
use miden_tx::{
    auth::TransactionAuthenticator,
    utils::{sync::RwLock, Deserializable, Serializable},
    AuthenticationError,
};
use rand::Rng;

pub struct ClientAuthenticator<R> {
    filepath: PathBuf,
    rng: Arc<RwLock<R>>,
}

impl<R: Rng> ClientAuthenticator<R> {
    pub fn new_with_rng(filepath: PathBuf, rng: R) -> Self {
        ClientAuthenticator {
            filepath,
            rng: Arc::new(RwLock::new(rng)),
        }
    }

    pub fn write_key_pairs(
        &self,
        key_pairs: BTreeMap<String, AuthSecretKey>,
    ) -> Result<(), AuthenticationError> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.filepath.clone())
            .unwrap();

        let mut writer = BufWriter::new(file);

        for (pub_key, secret_key) in key_pairs {
            let key_pair_hex = format!("{},{}\n", pub_key, hex::encode(secret_key.to_bytes()));
            writer.write_all(key_pair_hex.as_bytes()).unwrap();
        }

        Ok(())
    }

    pub fn read_key_pairs(&self) -> Result<BTreeMap<String, AuthSecretKey>, AuthenticationError> {
        if !self.filepath.exists() {
            return Ok(BTreeMap::new());
        }

        let file = OpenOptions::new().read(true).open(self.filepath.clone()).unwrap();

        let reader = BufReader::new(file);
        let mut key_pairs = BTreeMap::new();

        for line in reader.lines() {
            let line = line.unwrap();
            let mut parts = line.split(',');
            let pub_key = parts.next().unwrap();
            let secret_key_bytes = hex::decode(parts.next().unwrap()).unwrap();

            let secret_key = AuthSecretKey::read_from_bytes(secret_key_bytes.as_slice()).unwrap();

            key_pairs.insert(pub_key.to_string(), secret_key);
        }

        Ok(key_pairs)
    }

    pub fn add_key(&self, key: AuthSecretKey) {
        let mut key_pairs = self.read_key_pairs().unwrap();
        let pub_key = match &key {
            AuthSecretKey::RpoFalcon512(k) => Digest::from(Word::from(k.public_key())).to_hex(),
        };
        key_pairs.insert(pub_key, key);
        self.write_key_pairs(key_pairs).unwrap();
    }

    pub fn get_auth_by_pub_key(
        &self,
        pub_key: Word,
    ) -> Result<Option<AuthSecretKey>, AuthenticationError> {
        let key_pairs = self.read_key_pairs().unwrap();
        let pub_key_str = Digest::from(pub_key).to_hex();
        Ok(key_pairs.get(&pub_key_str).cloned())
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

        let secret_key = self
            .get_auth_by_pub_key(pub_key)
            .map_err(|err| {
                AuthenticationError::other_with_source("error getting secret key from file", err)
            })
            .unwrap();

        let AuthSecretKey::RpoFalcon512(k) = secret_key
            .ok_or(AuthenticationError::UnknownPublicKey(Digest::from(pub_key).into()))
            .unwrap();
        miden_tx::auth::signatures::get_falcon_signature(&k, message, &mut *rng)
    }
}

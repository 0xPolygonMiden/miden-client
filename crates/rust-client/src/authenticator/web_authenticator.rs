use alloc::{string::ToString, sync::Arc, vec::Vec};

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
use web_sys::wasm_bindgen::JsValue;

#[derive(Debug, Clone)]
pub struct WebAuthenticator<R> {
    rng: Arc<RwLock<R>>,
}

impl<R: Rng> WebAuthenticator<R> {
    pub fn new_with_rng(rng: R) -> Self {
        WebAuthenticator { rng: Arc::new(RwLock::new(rng)) }
    }

    pub fn add_key(&self, key: AuthSecretKey) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or(JsValue::from_str("Window not available"))?;
        let pub_key = match &key {
            AuthSecretKey::RpoFalcon512(k) => Digest::from(Word::from(k.public_key())).to_hex(),
        };

        let secret_key_hex = hex::encode(key.to_bytes());

        let storage = window
            .local_storage()?
            .ok_or(JsValue::from_str("Local storage not available"))?;

        storage.set_item(&pub_key, &secret_key_hex)?;

        Ok(())
    }

    pub fn get_auth_by_pub_key(&self, pub_key: Word) -> Result<Option<AuthSecretKey>, JsValue> {
        let window = web_sys::window().ok_or(JsValue::from_str("Window not available"))?;
        let pub_key_str = Digest::from(pub_key).to_hex();

        let storage = window
            .local_storage()?
            .ok_or(JsValue::from_str("Local storage not available"))?;
        let secret_key_hex = storage.get_item(&pub_key_str)?;

        match secret_key_hex {
            Some(secret_key_hex) => {
                let secret_key_bytes = hex::decode(secret_key_hex).map_err(|err| {
                    JsValue::from_str(&format!("error decoding secret key hex: {:?}", err))
                })?;

                let secret_key = AuthSecretKey::read_from_bytes(secret_key_bytes.as_slice())
                    .map_err(|err| {
                        JsValue::from_str(&format!("error reading secret key: {:?}", err))
                    })?;

                Ok(Some(secret_key))
            },
            None => Ok(None),
        }
    }
}

impl<R: Rng> TransactionAuthenticator for WebAuthenticator<R> {
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

        let secret_key = self.get_auth_by_pub_key(pub_key).map_err(|err| {
            AuthenticationError::other(
                err.as_string().unwrap_or_else(|| "Unknown error".to_string()),
            )
        })?;

        let AuthSecretKey::RpoFalcon512(k) = secret_key
            .ok_or(AuthenticationError::UnknownPublicKey(Digest::from(pub_key).into()))?;

        miden_tx::auth::signatures::get_falcon_signature(&k, message, &mut *rng)
    }
}

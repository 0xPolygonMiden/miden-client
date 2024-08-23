use miden_client::utils::Serializable;
use miden_objects::{
    accounts::AuthSecretKey as NativeAuthSecretKey, Felt as NativeFelt, Word as NativeWord,
};
use wasm_bindgen::prelude::*;

use super::{felt::Felt, word::Word};

#[derive(Clone, Debug)]
#[wasm_bindgen]
pub struct AuthSecretKey(NativeAuthSecretKey);

#[wasm_bindgen]
impl AuthSecretKey {
    pub fn get_rpo_falcon_512_public_key_as_word(&self) -> Word {
        let public_key = match self.0 {
            NativeAuthSecretKey::RpoFalcon512(ref key) => key.public_key(),
        };
        let public_key_as_native_word: NativeWord = public_key.into();
        public_key_as_native_word.into()
    }

    pub fn get_rpo_falcon_512_secret_key_as_felts(&self) -> Vec<Felt> {
        let secret_key_as_bytes = match self.0 {
            NativeAuthSecretKey::RpoFalcon512(ref key) => key.to_bytes(),
        };

        let secret_key_as_native_felts = secret_key_as_bytes
            .iter()
            .map(|a| NativeFelt::new(*a as u64))
            .collect::<Vec<NativeFelt>>();

        secret_key_as_native_felts.into_iter().map(Into::into).collect()
    }
}

// Conversions

impl From<NativeAuthSecretKey> for AuthSecretKey {
    fn from(native_auth_secret_key: NativeAuthSecretKey) -> Self {
        AuthSecretKey(native_auth_secret_key)
    }
}

impl From<&NativeAuthSecretKey> for AuthSecretKey {
    fn from(native_auth_secret_key: &NativeAuthSecretKey) -> Self {
        AuthSecretKey(native_auth_secret_key.clone())
    }
}

impl From<AuthSecretKey> for NativeAuthSecretKey {
    fn from(auth_secret_key: AuthSecretKey) -> Self {
        auth_secret_key.0
    }
}

impl From<&AuthSecretKey> for NativeAuthSecretKey {
    fn from(auth_secret_key: &AuthSecretKey) -> Self {
        auth_secret_key.0.clone()
    }
}

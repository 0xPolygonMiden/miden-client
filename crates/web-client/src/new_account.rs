use miden_client::{
    accounts::AccountType,
    auth::AuthSecretKey,
    crypto::SecretKey,
    utils::{create_basic_fungible_faucet, create_basic_wallet},
    Felt,
};
use miden_lib::AuthScheme;
use miden_objects::assets::TokenSymbol;
use wasm_bindgen::prelude::*;

use super::models::{account::Account, account_storage_mode::AccountStorageMode};
use crate::WebClient;

#[wasm_bindgen]
impl WebClient {
    pub async fn new_wallet(
        &mut self,
        storage_mode: &AccountStorageMode,
        mutable: bool,
    ) -> Result<Account, JsValue> {
        if let Some(client) = self.get_mut_inner() {
            let key_pair = SecretKey::with_rng(client.rng());

            let auth_scheme: AuthScheme =
                AuthScheme::RpoFalcon512 { pub_key: key_pair.public_key() };

            let mut init_seed = [0u8; 32];
            client.rng().fill_bytes(&mut init_seed);

            let account_type = if mutable {
                AccountType::RegularAccountUpdatableCode
            } else {
                AccountType::RegularAccountImmutableCode
            };

            let (new_account, seed) = match create_basic_wallet(
                init_seed,
                auth_scheme,
                account_type,
                storage_mode.into(),
            ) {
                Ok(result) => result,
                Err(err) => {
                    let error_message = format!("Failed to create new wallet: {:?}", err);
                    return Err(JsValue::from_str(&error_message));
                },
            };

            match client
                .insert_account(&new_account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair))
                .await
            {
                Ok(_) => Ok(new_account.into()),
                Err(err) => {
                    let error_message = format!("Failed to insert new wallet: {:?}", err);
                    Err(JsValue::from_str(&error_message))
                },
            }
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }

    pub async fn new_faucet(
        &mut self,
        storage_mode: &AccountStorageMode,
        non_fungible: bool,
        token_symbol: &str,
        decimals: u8,
        max_supply: u64,
    ) -> Result<Account, JsValue> {
        if non_fungible {
            return Err(JsValue::from_str("Non-fungible faucets are not supported yet"));
        }

        if let Some(client) = self.get_mut_inner() {
            let key_pair = SecretKey::with_rng(client.rng());

            let auth_scheme: AuthScheme =
                AuthScheme::RpoFalcon512 { pub_key: key_pair.public_key() };

            let mut init_seed = [0u8; 32];
            client.rng().fill_bytes(&mut init_seed);

            let (new_account, seed) = match create_basic_fungible_faucet(
                init_seed,
                TokenSymbol::new(token_symbol).map_err(|e| JsValue::from_str(&e.to_string()))?,
                decimals,
                Felt::try_from(max_supply.to_le_bytes().as_slice())
                    .expect("u64 can be safely converted to a field element"),
                storage_mode.into(),
                auth_scheme,
            ) {
                Ok(result) => result,
                Err(err) => {
                    let error_message = format!("Failed to create new faucet: {:?}", err);
                    return Err(JsValue::from_str(&error_message));
                },
            };

            match client
                .insert_account(&new_account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair))
                .await
            {
                Ok(_) => Ok(new_account.into()),
                Err(err) => {
                    let error_message = format!("Failed to insert new faucet: {:?}", err);
                    Err(JsValue::from_str(&error_message))
                },
            }
        } else {
            Err(JsValue::from_str("Client not initialized"))
        }
    }
}

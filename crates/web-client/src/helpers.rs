use miden_client::{
    account::{Account, AccountBuilder, AccountType},
    crypto::{RpoRandomCoin, SecretKey},
    Client,
};
use miden_lib::account::{auth::RpoFalcon512, wallets::BasicWallet};
use miden_objects::Felt;
use rand::{rngs::StdRng, Rng, SeedableRng};
use wasm_bindgen::JsValue;

use crate::models::account_storage_mode::AccountStorageMode;

pub async fn generate_account(
    client: &mut Client<RpoRandomCoin>,
    storage_mode: &AccountStorageMode,
    mutable: bool,
    seed: Option<Vec<u8>>,
) -> Result<(Account, [Felt; 4], SecretKey), JsValue> {
    let mut rng = match seed {
        Some(seed_bytes) => {
            if seed_bytes.len() == 32 {
                let mut seed_array = [0u8; 32];
                seed_array.copy_from_slice(&seed_bytes);
                let mut std_rng = StdRng::from_seed(seed_array);
                let coin_seed: [u64; 4] = std_rng.gen();
                &mut RpoRandomCoin::new(coin_seed.map(Felt::new))
            } else {
                Err(JsValue::from_str("Seed must be exactly 32 bytes"))?
            }
        },
        None => client.rng(),
    };
    let key_pair = SecretKey::with_rng(&mut rng);

    let mut init_seed = [0u8; 32];
    rng.fill_bytes(&mut init_seed);

    let account_type = if mutable {
        AccountType::RegularAccountUpdatableCode
    } else {
        AccountType::RegularAccountImmutableCode
    };

    // Using the genesis block as the anchor for now to ensure deterministic outcomes
    let anchor_block = client
        .ensure_genesis_in_place()
        .await
        .map_err(|err| JsValue::from_str(&format!("Failed to create new wallet: {err:?}")))?;

    let (new_account, account_seed) = match AccountBuilder::new(init_seed)
        .anchor((&anchor_block).try_into().unwrap())
        .account_type(account_type)
        .storage_mode(storage_mode.into())
        .with_component(RpoFalcon512::new(key_pair.public_key()))
        .with_component(BasicWallet)
        .build()
    {
        Ok(result) => result,
        Err(err) => {
            let error_message = format!("Failed to create new wallet: {err:?}");
            return Err(JsValue::from_str(&error_message));
        },
    };

    Ok((new_account, account_seed, key_pair))
}

extern crate alloc;
use alloc::sync::Arc;

use console_error_panic_hook::set_once;
use miden_client::{
    authenticator::{keystore::WebKeyStore, ClientAuthenticator},
    rpc::WebTonicRpcClient,
    store::web_store::WebStore,
    sync::{on_note_received, StateSync},
    Client,
};
use miden_objects::{crypto::rand::RpoRandomCoin, Felt};
use miden_proving_service_client::proving_service::tx_prover::RemoteTransactionProver;
use rand::{rngs::StdRng, Rng, SeedableRng};
use wasm_bindgen::prelude::*;

pub mod account;
pub mod export;
pub mod import;
pub mod models;
pub mod new_account;
pub mod new_transactions;
pub mod notes;
pub mod sync;
pub mod tags;
pub mod transactions;

#[wasm_bindgen]
pub struct WebClient {
    store: Option<Arc<WebStore>>,
    remote_prover: Option<Arc<RemoteTransactionProver>>,
    keystore: Option<WebKeyStore>,
    inner: Option<Client<RpoRandomCoin>>,
}

impl Default for WebClient {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WebClient {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        set_once();
        WebClient {
            inner: None,
            remote_prover: None,
            store: None,
            keystore: None,
        }
    }

    pub(crate) fn get_mut_inner(&mut self) -> Option<&mut Client<RpoRandomCoin>> {
        self.inner.as_mut()
    }

    pub async fn create_client(
        &mut self,
        node_url: Option<String>,
        prover_url: Option<String>,
        seed: Option<Vec<u8>>,
    ) -> Result<JsValue, JsValue> {
        let mut rng = match seed {
            Some(seed_bytes) => {
                if seed_bytes.len() == 32 {
                    let mut seed_array = [0u8; 32];
                    seed_array.copy_from_slice(&seed_bytes);
                    StdRng::from_seed(seed_array)
                } else {
                    return Err(JsValue::from_str("Seed must be exactly 32 bytes"));
                }
            },
            None => StdRng::from_entropy(),
        };
        let coin_seed: [u64; 4] = rng.gen();

        let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));
        let web_store: WebStore = WebStore::new()
            .await
            .map_err(|_| JsValue::from_str("Failed to initialize WebStore"))?;
        let web_store = Arc::new(web_store);

        let keystore = WebKeyStore {};

        let authenticator = Arc::new(ClientAuthenticator::new(rng, keystore.clone()));
        let web_rpc_client = Arc::new(WebTonicRpcClient::new(
            &node_url.unwrap_or_else(|| miden_client::rpc::Endpoint::testnet().to_string()),
        ));

        self.remote_prover =
            prover_url.map(|prover_url| Arc::new(RemoteTransactionProver::new(prover_url)));

        let state_sync_component = StateSync::new(
            web_rpc_client.clone(),
            Box::new({
                let store_clone = web_store.clone();
                move |committed_note, public_note| {
                    Box::pin(on_note_received(store_clone.clone(), committed_note, public_note))
                }
            }),
            Box::new(move |_committed_note| Box::pin(async { Ok(true) })),
        );

        self.inner = Some(Client::new(
            web_rpc_client,
            rng,
            web_store.clone(),
            authenticator,
            state_sync_component,
            false,
        ));
        self.store = Some(web_store);
        self.keystore = Some(keystore);

        Ok(JsValue::from_str("Client created successfully"))
    }
}

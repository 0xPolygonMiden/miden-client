use alloc::sync::Arc;

use miden_client::{
    transaction::{LocalTransactionProver, TransactionProver as TransactionProverTrait},
    RemoteTransactionProver,
};
use miden_tx::ProvingOptions;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct TransactionProver {
    prover: Arc<dyn TransactionProverTrait>,
}

#[wasm_bindgen]
impl TransactionProver {
    #[wasm_bindgen(js_name = "newLocalProver")]
    pub fn new_local_prover() -> TransactionProver {
        let local_prover = LocalTransactionProver::new(ProvingOptions::default());
        TransactionProver { prover: Arc::new(local_prover) }
    }

    #[wasm_bindgen(js_name = "newRemoteProver")]
    pub fn new_remote_prover(endpoint: &str) -> TransactionProver {
        let remote_prover = RemoteTransactionProver::new(endpoint);
        TransactionProver { prover: Arc::new(remote_prover) }
    }
}

impl TransactionProver {
    pub fn get_prover(&self) -> Arc<dyn TransactionProverTrait> {
        self.prover.clone()
    }
}

use alloc::sync::Arc;
use miden_remote_provers::RemoteTransactionProver;
use miden_tx::{LocalTransactionProver, TransactionProver};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ProverWrapper {
    prover: Arc<dyn TransactionProver>,
}

#[wasm_bindgen]
impl ProverWrapper {
    pub fn new_local_prover() -> ProverWrapper {
        let local_prover = LocalTransactionProver::new(Default::default());
        ProverWrapper {
            prover: Arc::new(local_prover),
        }
    }

    pub fn new_remote_prover(endpoint: &str) -> ProverWrapper {
        let remote_prover = RemoteTransactionProver::new(endpoint);
        ProverWrapper {
            prover: Arc::new(remote_prover),
        }
    }
}

impl ProverWrapper {
    pub fn get_prover(&self) -> Arc<dyn TransactionProver> {
        self.prover.clone()
    }
}

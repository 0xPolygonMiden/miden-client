use alloc::sync::Arc;

use miden_client::transaction::{
    LocalTransactionProver, TransactionProver as TransactionProverTrait,
};
use miden_proving_service_client::RemoteTransactionProver;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct TransactionProver {
    prover: Arc<dyn TransactionProverTrait>,
    endpoint: Option<String>,
}

#[wasm_bindgen]
impl TransactionProver {
    pub fn new_local_prover() -> TransactionProver {
        let local_prover = LocalTransactionProver::new(Default::default());
        TransactionProver { prover: Arc::new(local_prover), endpoint: None }
    }

    pub fn new_remote_prover(endpoint: &str) -> TransactionProver {
        let remote_prover = RemoteTransactionProver::new(endpoint);
        TransactionProver { prover: Arc::new(remote_prover), endpoint: Some(endpoint.to_string()) }
    }

    pub fn from_str(prover_type: &str, endpoint: Option<String>) -> TransactionProver {
        match prover_type {
            "local" => TransactionProver::new_local_prover(),
            "remote" => {
                // Use as_deref() to convert Option<String> to Option<&str>
                let ep = endpoint.as_deref().unwrap_or("http://localhost:8080");
                TransactionProver::new_remote_prover(ep)
            },
            _ => panic!("Invalid prover type"), // Consider better error handling in production.
        }
    }

    pub fn as_str(&self) -> String {
        match &self.endpoint {
            Some(ep) => format!("remote:{}", ep),
            None => "local".to_string(),
        }
    }

    pub fn endpoint(&self) -> Option<String> {
        self.endpoint.clone()
    }
}

impl TransactionProver {
    pub fn get_prover(&self) -> Arc<dyn TransactionProverTrait> {
        self.prover.clone()
    }
}

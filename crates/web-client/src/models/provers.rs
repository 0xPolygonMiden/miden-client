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
        TransactionProver {
            prover: Arc::new(local_prover),
            endpoint: None,
        }
    }

    pub fn new_remote_prover(endpoint: &str) -> TransactionProver {
        let remote_prover = RemoteTransactionProver::new(endpoint);
        TransactionProver {
            prover: Arc::new(remote_prover),
            endpoint: Some(endpoint.to_string()),
        }
    }

    pub fn serialize(&self) -> String {
        match &self.endpoint {
            Some(ep) => format!("remote:{}", ep),
            None => "local".to_string(),
        }
    }

    pub fn deserialize(
        prover_type: &str,
        endpoint: Option<String>,
    ) -> Result<TransactionProver, JsValue> {
        match prover_type {
            "local" => Ok(TransactionProver::new_local_prover()),
            "remote" => {
                if let Some(ep) = endpoint {
                    Ok(TransactionProver::new_remote_prover(&ep))
                } else {
                    Err(JsValue::from_str("Remote prover requires an endpoint"))
                }
            },
            _ => Err(JsValue::from_str("Invalid prover type")),
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

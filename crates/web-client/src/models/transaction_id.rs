use miden_objects::transaction::TransactionId as NativeTransactionId;
use wasm_bindgen::prelude::*;

use super::{felt::Felt, rpo_digest::RpoDigest};

#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionId(NativeTransactionId);

#[wasm_bindgen]
impl TransactionId {
    pub fn as_elements(&self) -> Vec<Felt> {
        self.0.as_elements().iter().map(|e| e.into()).collect()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.as_bytes().to_vec()
    }

    pub fn to_hex(&self) -> String {
        self.0.to_hex()
    }

    pub fn inner(&self) -> RpoDigest {
        self.0.inner().into()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeTransactionId> for TransactionId {
    fn from(native_id: NativeTransactionId) -> Self {
        TransactionId(native_id)
    }
}

impl From<&NativeTransactionId> for TransactionId {
    fn from(native_id: &NativeTransactionId) -> Self {
        TransactionId(native_id.clone())
    }
}

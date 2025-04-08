use miden_client::transaction::TransactionRequest as NativeTransactionRequest;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::Uint8Array;

use crate::utils::{deserialize_from_uint8array, serialize_to_uint8array};

pub mod note_and_args;
pub mod note_details_and_tag;
pub mod note_id_and_args;
pub mod transaction_request_builder;

#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionRequest(NativeTransactionRequest);

#[wasm_bindgen]
impl TransactionRequest {
    pub fn serialize(&self) -> Uint8Array {
        serialize_to_uint8array(&self.0)
    }

    pub fn deserialize(bytes: &Uint8Array) -> Result<TransactionRequest, JsValue> {
        deserialize_from_uint8array::<NativeTransactionRequest>(bytes).map(TransactionRequest)
    }
}

// CONVERSIONS
// ================================================================================================

impl From<TransactionRequest> for NativeTransactionRequest {
    fn from(transaction_request: TransactionRequest) -> Self {
        transaction_request.0
    }
}

impl From<&TransactionRequest> for NativeTransactionRequest {
    fn from(transaction_request: &TransactionRequest) -> Self {
        transaction_request.0.clone()
    }
}

impl From<NativeTransactionRequest> for TransactionRequest {
    fn from(transaction_request: NativeTransactionRequest) -> Self {
        TransactionRequest(transaction_request)
    }
}

impl From<&NativeTransactionRequest> for TransactionRequest {
    fn from(transaction_request: &NativeTransactionRequest) -> Self {
        TransactionRequest(transaction_request.clone())
    }
}

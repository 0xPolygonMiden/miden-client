use miden_client::sync::SyncSummary as NativeSyncSummary;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::Uint8Array;

use crate::{
    models::{account_id::AccountId, note_id::NoteId, transaction_id::TransactionId},
    utils::*,
};

#[wasm_bindgen]
pub struct SyncSummary(NativeSyncSummary);

#[wasm_bindgen]
impl SyncSummary {
    pub fn block_num(&self) -> u32 {
        self.0.block_num.as_u32()
    }

    pub fn received_notes(&self) -> Vec<NoteId> {
        self.0.received_notes.iter().map(|note_id| note_id.into()).collect()
    }

    pub fn committed_notes(&self) -> Vec<NoteId> {
        self.0.committed_notes.iter().map(|note_id| note_id.into()).collect()
    }

    pub fn consumed_notes(&self) -> Vec<NoteId> {
        self.0.consumed_notes.iter().map(|note_id| note_id.into()).collect()
    }

    pub fn updated_accounts(&self) -> Vec<AccountId> {
        self.0.updated_accounts.iter().map(|account_id| account_id.into()).collect()
    }

    pub fn committed_transactions(&self) -> Vec<TransactionId> {
        self.0
            .committed_transactions
            .iter()
            .map(|transaction_id| transaction_id.into())
            .collect()
    }

    pub fn serialize(&self) -> Uint8Array {
        serialize_to_uint8array(&self.0)
    }

    pub fn deserialize(bytes: Uint8Array) -> Result<SyncSummary, JsValue> {
        deserialize_from_uint8array::<NativeSyncSummary>(bytes).map(SyncSummary)
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeSyncSummary> for SyncSummary {
    fn from(native_sync_summary: NativeSyncSummary) -> Self {
        SyncSummary(native_sync_summary)
    }
}

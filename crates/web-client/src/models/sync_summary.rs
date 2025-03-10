use miden_client::sync::SyncSummary as NativeSyncSummary;
use wasm_bindgen::prelude::*;

use super::{account_id::AccountId, note_id::NoteId, transaction_id::TransactionId};

#[wasm_bindgen]
pub struct SyncSummary(NativeSyncSummary);

#[wasm_bindgen]
impl SyncSummary {
    #[wasm_bindgen(js_name = "blockNum")]
    pub fn block_num(&self) -> u32 {
        self.0.block_num.as_u32()
    }

    #[wasm_bindgen(js_name = "receivedNotes")]
    pub fn received_notes(&self) -> Vec<NoteId> {
        self.0.received_notes.iter().map(Into::into).collect()
    }

    #[wasm_bindgen(js_name = "committedNotes")]
    pub fn committed_notes(&self) -> Vec<NoteId> {
        self.0.committed_notes.iter().map(Into::into).collect()
    }

    #[wasm_bindgen(js_name = "consumedNotes")]
    pub fn consumed_notes(&self) -> Vec<NoteId> {
        self.0.consumed_notes.iter().map(Into::into).collect()
    }

    #[wasm_bindgen(js_name = "updatedAccounts")]
    pub fn updated_accounts(&self) -> Vec<AccountId> {
        self.0.updated_accounts.iter().map(Into::into).collect()
    }

    #[wasm_bindgen(js_name = "committedTransactions")]
    pub fn committed_transactions(&self) -> Vec<TransactionId> {
        self.0.committed_transactions.iter().map(Into::into).collect()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeSyncSummary> for SyncSummary {
    fn from(native_sync_summary: NativeSyncSummary) -> Self {
        SyncSummary(native_sync_summary)
    }
}

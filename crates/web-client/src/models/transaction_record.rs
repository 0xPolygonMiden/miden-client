use miden_client::transactions::TransactionRecord as NativeTransactionRecord;
use wasm_bindgen::prelude::*;

use super::{
    account_id::AccountId, output_notes::OutputNotes, rpo_digest::RpoDigest,
    transaction_id::TransactionId, transaction_status::TransactionStatus,
};

#[derive(Clone)]
#[wasm_bindgen]
pub struct TransactionRecord(NativeTransactionRecord);

#[wasm_bindgen]
impl TransactionRecord {
    pub fn id(&self) -> TransactionId {
        self.0.id.into()
    }

    pub fn account_id(&self) -> AccountId {
        self.0.account_id.into()
    }

    pub fn init_account_state(&self) -> RpoDigest {
        self.0.init_account_state.into()
    }

    pub fn final_account_state(&self) -> RpoDigest {
        self.0.final_account_state.into()
    }

    pub fn input_note_nullifiers(&self) -> Vec<RpoDigest> {
        self.0
            .input_note_nullifiers
            .iter()
            .map(|rpo_digest| rpo_digest.into())
            .collect()
    }

    pub fn output_notes(&self) -> OutputNotes {
        self.0.output_notes.clone().into()
    }

    // pub fn transaction_script(&self) -> Option<TransactionScript> {
    //     self.0.transaction_script.map(|script| script.into())
    // }

    pub fn block_num(&self) -> u32 {
        self.0.block_num
    }

    pub fn transaction_status(&self) -> TransactionStatus {
        self.0.transaction_status.clone().into()
    }
}

// CONVERSIONS
// ================================================================================================

impl From<NativeTransactionRecord> for TransactionRecord {
    fn from(native_record: NativeTransactionRecord) -> Self {
        TransactionRecord(native_record)
    }
}

impl From<&NativeTransactionRecord> for TransactionRecord {
    fn from(native_record: &NativeTransactionRecord) -> Self {
        TransactionRecord(native_record.clone())
    }
}

use miden_client::transaction::TransactionRecord as NativeTransactionRecord;
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

    #[wasm_bindgen(js_name = "accountId")]
    pub fn account_id(&self) -> AccountId {
        self.0.details.account_id.into()
    }

    #[wasm_bindgen(js_name = "initAccountState")]
    pub fn init_account_state(&self) -> RpoDigest {
        self.0.details.init_account_state.into()
    }

    #[wasm_bindgen(js_name = "finalAccountState")]
    pub fn final_account_state(&self) -> RpoDigest {
        self.0.details.final_account_state.into()
    }

    #[wasm_bindgen(js_name = "inputNoteNullifiers")]
    pub fn input_note_nullifiers(&self) -> Vec<RpoDigest> {
        self.0.details.input_note_nullifiers.iter().map(Into::into).collect()
    }

    #[wasm_bindgen(js_name = "outputNotes")]
    pub fn output_notes(&self) -> OutputNotes {
        self.0.details.output_notes.clone().into()
    }

    // pub fn transaction_script(&self) -> Option<TransactionScript> {
    //     self.0.transaction_script.map(|script| script.into())
    // }

    #[wasm_bindgen(js_name = "blockNum")]
    pub fn block_num(&self) -> u32 {
        self.0.details.block_num.as_u32()
    }

    #[wasm_bindgen(js_name = "transactionStatus")]
    pub fn transaction_status(&self) -> TransactionStatus {
        self.0.status.clone().into()
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

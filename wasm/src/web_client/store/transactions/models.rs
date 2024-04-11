use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TransactionIdxdbObject {
    pub id: String,
    pub account_id: String, // usually i64
    pub init_account_state: String,
    pub final_account_state: String,
    pub input_notes: String,
    pub output_notes: Vec<u8>,
    pub script_hash: Option<Vec<u8>>,
    pub script_program: Option<Vec<u8>>,
    pub script_inputs: Option<String>,
    pub block_num: String, // usually u32
    pub commit_height: Option<String> // usually Option<u32>
}
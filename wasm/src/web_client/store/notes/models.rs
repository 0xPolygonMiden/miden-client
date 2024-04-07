use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct InputNoteIdxdbObject {
    pub assets: Vec<u8>,
    pub details: String,
    pub metadata: String,
    pub inclusion_proof: Option<String>
}

#[derive(Serialize, Deserialize)]
pub struct OutputNoteIdxdbObject {
    pub assets: Vec<u8>,
    pub details: String,
    pub metadata: String,
    pub inclusion_proof: Option<String>
}
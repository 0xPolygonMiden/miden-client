use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct InputNoteIdxdbObject {
    pub assets: Vec<u8>,
    pub details: String,
    pub recipient: String,
    pub status: String,
    pub metadata: Option<String>,
    pub inclusion_proof: Option<String>,
    pub serialized_note_script: Vec<u8>
}

#[derive(Serialize, Deserialize)]
pub struct OutputNoteIdxdbObject {
    pub assets: Vec<u8>,
    pub details: Option<String>,
    pub recipient: String,
    pub status: String,
    pub metadata: String,
    pub inclusion_proof: Option<String>,
    pub serialized_note_script: Option<Vec<u8>>
}
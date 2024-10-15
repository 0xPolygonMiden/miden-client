use alloc::{string::String, vec::Vec};

use base64::{engine::general_purpose, Engine as _};
use serde::{de::Error, Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize)]
pub struct InputNoteIdxdbObject {
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub assets: Vec<u8>,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub serial_number: Vec<u8>,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub inputs: Vec<u8>,
    pub created_at: String,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub serialized_note_script: Vec<u8>,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub state: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct OutputNoteIdxdbObject {
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub assets: Vec<u8>,
    pub recipient_digest: String,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub metadata: Vec<u8>,
    pub after_block_height: Option<u32>,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub state: Vec<u8>,
}

fn base64_to_vec_u8_required<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let base64_str: String = Deserialize::deserialize(deserializer)?;
    general_purpose::STANDARD
        .decode(&base64_str)
        .map_err(|e| Error::custom(format!("Base64 decode error: {}", e)))
}

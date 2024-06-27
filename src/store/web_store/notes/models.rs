use base64::decode as base64_decode;
use serde::{de::Error, Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize)]
pub struct InputNoteIdxdbObject {
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub assets: Vec<u8>,
    pub details: String,
    pub recipient: String,
    pub status: String,
    pub metadata: Option<String>,
    pub inclusion_proof: Option<String>,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub serialized_note_script: Vec<u8>,
    pub consumer_account_id: Option<String>,
    pub created_at: String,
    pub submitted_at: Option<String>,
    pub nullifier_height: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct OutputNoteIdxdbObject {
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub assets: Vec<u8>,
    pub details: Option<String>,
    pub recipient: String,
    pub status: String,
    pub metadata: String,
    pub inclusion_proof: Option<String>,
    #[serde(deserialize_with = "base64_to_vec_u8_optional", default)]
    pub serialized_note_script: Option<Vec<u8>>,
    pub consumer_account_id: Option<String>,
    pub created_at: String,
    pub submitted_at: Option<String>,
    pub nullifier_height: Option<String>,
}

fn base64_to_vec_u8_required<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let base64_str: String = Deserialize::deserialize(deserializer)?;
    base64_decode(&base64_str).map_err(|e| Error::custom(format!("Base64 decode error: {}", e)))
}

fn base64_to_vec_u8_optional<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: Deserializer<'de>,
{
    let base64_str: Option<String> = Option::deserialize(deserializer)?;
    match base64_str {
        Some(str) => base64_decode(&str)
            .map(Some)
            .map_err(|e| Error::custom(format!("Base64 decode error: {}", e))),
        None => Ok(None),
    }
}

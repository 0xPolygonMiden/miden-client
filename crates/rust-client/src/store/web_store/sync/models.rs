use alloc::{string::String, vec::Vec};

use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Deserializer, Serialize, de::Error};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncHeightIdxdbObject {
    pub block_num: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteTagIdxdbObject {
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub tag: Vec<u8>,
    pub source_note_id: Option<String>,
    pub source_account_id: Option<String>,
}

fn base64_to_vec_u8_required<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let base64_str: String = Deserialize::deserialize(deserializer)?;
    general_purpose::STANDARD
        .decode(&base64_str)
        .map_err(|e| Error::custom(format!("Base64 decode error: {e}")))
}

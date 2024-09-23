use alloc::{string::String, vec::Vec};

use base64::{engine::general_purpose, Engine as _};
use serde::{de::Error, Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SyncHeightIdxdbObject {
    pub block_num: String,
}

#[derive(Serialize, Deserialize)]
pub struct NoteTagsIdxdbObject {
    #[serde(deserialize_with = "base64_to_vec_u8_optional", default)]
    pub tags: Option<Vec<u8>>,
}

fn base64_to_vec_u8_optional<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: Deserializer<'de>,
{
    let base64_str: Option<String> = Option::deserialize(deserializer)?;
    match base64_str {
        Some(str) => general_purpose::STANDARD
            .decode(&str)
            .map(Some)
            .map_err(|e| Error::custom(format!("Base64 decode error: {}", e))),
        None => Ok(None),
    }
}

use alloc::{string::String, vec::Vec};

use base64::{engine::general_purpose, Engine as _};
use serde::{de::Error, Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BlockHeaderIdxdbObject {
    pub block_num: String,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub header: Vec<u8>,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub chain_mmr: Vec<u8>,
    pub has_client_notes: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ChainMmrNodeIdxdbObject {
    pub id: String,
    pub node: String,
}

#[derive(Serialize, Deserialize)]
pub struct MmrPeaksIdxdbObject {
    #[serde(deserialize_with = "base64_to_vec_u8_optional", default)]
    pub peaks: Option<Vec<u8>>,
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

fn base64_to_vec_u8_optional<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: Deserializer<'de>,
{
    let base64_str: Option<String> = Option::deserialize(deserializer)?;
    match base64_str {
        Some(str) => general_purpose::STANDARD
            .decode(&str)
            .map(Some)
            .map_err(|e| Error::custom(format!("Base64 decode error: {e}"))),
        None => Ok(None),
    }
}

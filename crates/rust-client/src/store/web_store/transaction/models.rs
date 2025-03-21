use alloc::{string::String, vec::Vec};

use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Deserializer, Serialize, de::Error};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionIdxdbObject {
    pub id: String,
    pub account_id: String, // usually i64
    pub init_account_state: String,
    pub final_account_state: String,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub input_notes: Vec<u8>,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub output_notes: Vec<u8>,
    #[serde(deserialize_with = "base64_to_vec_u8_optional", default)]
    pub script_root: Option<Vec<u8>>,
    #[serde(deserialize_with = "base64_to_vec_u8_optional", default)]
    pub tx_script: Option<Vec<u8>>,
    pub block_num: String,             // usually u32
    pub commit_height: Option<String>, // usually Option<u32>
    pub discarded: bool,
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

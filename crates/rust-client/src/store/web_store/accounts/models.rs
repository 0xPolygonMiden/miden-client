use alloc::{string::String, vec::Vec};

use base64::decode as base64_decode;
use serde::{de::Error, Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AccountCodeIdxdbObject {
    pub root: String,
    pub procedures: String,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub module: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct AccountAuthIdxdbObject {
    pub id: String,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub auth_info: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct AccountStorageIdxdbObject {
    pub root: String,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub storage: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct AccountVaultIdxdbObject {
    pub root: String,
    pub assets: String,
}

#[derive(Serialize, Deserialize)]
pub struct AccountRecordIdxdbOjbect {
    pub id: String,
    pub nonce: String,
    pub vault_root: String,
    pub storage_root: String,
    pub code_root: String,
    #[serde(deserialize_with = "base64_to_vec_u8_optional", default)]
    pub account_seed: Option<Vec<u8>>,
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

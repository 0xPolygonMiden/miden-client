use alloc::{string::String, vec::Vec};

use base64::{engine::general_purpose, Engine as _};
use serde::{de::Error, Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AccountCodeIdxdbObject {
    pub root: String,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub code: Vec<u8>,
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
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub assets: Vec<u8>,
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

#[derive(Serialize, Deserialize)]
pub struct ForeignAcountCodeIdxdbObject {
    pub account_id: String,
    #[serde(deserialize_with = "base64_to_vec_u8_required", default)]
    pub code: Vec<u8>,
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

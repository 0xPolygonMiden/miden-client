use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AccountCodeIdxdbObject {
    pub root: String,
    pub procedures: String,
    pub module: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct AccountAuthIdxdbObject {
    pub id: String,
    pub auth_info: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct AccountStorageIdxdbObject {
    pub root: String,
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
    // Use Vec<u8> to represent the Blob data in Rust. Conversion will be handled in JS.
    pub account_seed: Option<Vec<u8>>,
}
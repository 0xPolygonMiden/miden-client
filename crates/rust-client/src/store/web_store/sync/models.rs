use alloc::{string::String, vec::Vec};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SyncHeightIdxdbObject {
    pub block_num: String,
}

#[derive(Serialize, Deserialize)]
pub struct NoteTagsIdxdbObject {
    pub tags: Vec<u8>,
}

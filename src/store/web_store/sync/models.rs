use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SyncHeightIdxdbObject {
    pub block_num: String,
}

#[derive(Serialize, Deserialize)]
pub struct NoteTagsIdxdbObject {
    pub tags: String,
}

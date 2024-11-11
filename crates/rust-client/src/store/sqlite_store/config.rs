// STORE CONFIG
// ================================================================================================

use alloc::string::{String, ToString};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqliteStoreConfig {
    pub database_filepath: String,
}

impl TryFrom<&str> for SqliteStoreConfig {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        SqliteStoreConfig::try_from(value.to_string())
    }
}

// TODO: Implement error checking for invalid paths, or make it based on Path types
impl TryFrom<String> for SqliteStoreConfig {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self { database_filepath: value })
    }
}

impl Default for SqliteStoreConfig {
    fn default() -> Self {
        const STORE_FILENAME: &str = "store.sqlite3";

        // Get current directory
        let exec_dir = PathBuf::new();

        // Append filepath
        let database_filepath = exec_dir
            .join(STORE_FILENAME)
            .into_os_string()
            .into_string()
            .expect("Creating the hardcoded store path should not panic");

        Self { database_filepath }
    }
}

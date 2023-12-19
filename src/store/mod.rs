use crate::config::StoreConfig;
use crate::errors::StoreError;

use clap::error::Result;
use rusqlite::Connection;

pub mod accounts;
mod migrations;
pub mod input_notes;
pub mod output_notes;
pub mod transactions;

pub(crate) mod mock_executor_data_store;

// CLIENT STORE
// ================================================================================================

pub struct Store {
    db: Connection,
}

impl Store {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Store] instantiated with the specified configuration options.
    pub fn new(config: StoreConfig) -> Result<Self, StoreError> {
        let mut db = Connection::open(config.path).map_err(StoreError::ConnectionError)?;
        migrations::update_to_latest(&mut db)?;

        Ok(Self { db })
    }
}


// NOTE FILTERs
// ================================================================================================
/// Represents a filter for input notes
pub enum NoteFilter {
    All,
    Consumed,
    Committed,
    Pending,
}

impl NoteFilter {
    pub fn to_query(&self) -> String {
        let base = String::from("SELECT script, inputs, vault, serial_num, sender_id, tag, num_assets, inclusion_proof FROM input_notes");
        match self {
            NoteFilter::All => base,
            NoteFilter::Committed => format!("{base} WHERE status = 'committed'"),
            NoteFilter::Consumed => format!("{base} WHERE status = 'consumed'"),
            NoteFilter::Pending => format!("{base} WHERE status = 'pending'"),
        }
    }
}


// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use std::env::temp_dir;
    use uuid::Uuid;

    use rusqlite::Connection;

    use super::{migrations, Store};

    pub fn create_test_store_path() -> std::path::PathBuf {
        let mut temp_file = temp_dir();
        temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
        temp_file
    }

    pub fn create_test_store() -> Store {
        let temp_file = create_test_store_path();
        let mut db = Connection::open(temp_file).unwrap();
        migrations::update_to_latest(&mut db).unwrap();

        Store { db }
    }
}

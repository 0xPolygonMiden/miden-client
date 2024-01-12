use crate::{config::StoreConfig, errors::StoreError};

use clap::error::Result;
use crypto::merkle::Mmr;
use rusqlite::Connection;

pub mod accounts;
pub mod chain_data;
mod migrations;
pub mod notes;
pub mod state_sync;
pub mod transactions;

#[cfg(any(test, feature = "mock"))]
pub mod mock_executor_data_store;

pub mod data_store;

// CLIENT STORE
// ================================================================================================

pub struct Store {
    pub(crate) db: Connection,
    pub(crate) mmr: Mmr,
}

impl Store {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Store] instantiated with the specified configuration options.
    pub fn new(config: StoreConfig) -> Result<Self, StoreError> {
        let mut db = Connection::open(config.path).map_err(StoreError::ConnectionError)?;
        migrations::update_to_latest(&mut db)?;
        let mmr = Mmr::new();

        Ok(Self { db, mmr })
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use crypto::merkle::Mmr;
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

        Store {
            db,
            mmr: Mmr::new(),
        }
    }
}

use crate::{config::StoreConfig, errors::StoreError};

use clap::error::Result;
use crypto::utils::DeserializationError;
use rusqlite::Connection;

pub mod accounts;
pub mod chain_data;
mod migrations;
pub mod notes;
pub mod sync;
pub mod transactions;

#[cfg(any(test, feature = "mock"))]
pub mod mock_executor_data_store;

pub mod data_store;

// CLIENT STORE
// ================================================================================================

pub struct Store {
    pub(crate) db: Connection,
}

impl Store {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Store] instantiated with the specified configuration options.
    pub fn new(config: StoreConfig) -> Result<Self, StoreError> {
        let mut db = Connection::open(config.database_filepath)?;
        migrations::update_to_latest(&mut db)?;

        Ok(Self { db })
    }
}

// HELPER Functions
// ================================================================================================

pub(crate) fn parse_json_array(array_as_str: String) -> Vec<String> {
    let array_as_str = array_as_str.replace(['[', ']', '\"'], "");

    // If the string is empty `split` actually yields an empty string instead of an empty
    // iterator chain so we need to take care of it
    if array_as_str.is_empty() {
        Vec::new()
    } else {
        array_as_str
            .split(',')
            .map(|str| str.to_string())
            .collect::<Vec<_>>()
    }
}

pub(crate) fn parse_json_byte_str(byte_as_str: String) -> Result<u8, StoreError> {
    byte_as_str.parse().map_err(|_err| {
        StoreError::DataDeserializationError(DeserializationError::InvalidValue(
            byte_as_str.to_string(),
        ))
    })
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use std::env::temp_dir;
    use uuid::Uuid;

    use rusqlite::Connection;

    use crate::{
        client::Client,
        config::{ClientConfig, RpcConfig},
    };

    use super::{migrations, Store};

    pub fn create_test_client() -> Client {
        let client_config = ClientConfig {
            store: create_test_store_path()
                .into_os_string()
                .into_string()
                .unwrap()
                .try_into()
                .unwrap(),
            rpc: RpcConfig::default(),
        };

        Client::new(client_config).unwrap()
    }

    pub(crate) fn create_test_store_path() -> std::path::PathBuf {
        let mut temp_file = temp_dir();
        temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
        temp_file
    }

    pub(crate) fn create_test_store() -> Store {
        let temp_file = create_test_store_path();
        let mut db = Connection::open(temp_file).unwrap();
        migrations::update_to_latest(&mut db).unwrap();

        Store { db }
    }
}

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

///
/// Represents a connection with an sqlite database
///
///
/// Current table definitions can be found at `store.sql` migration file. One particular column
/// type used is JSON, for which you can look more info at [sqlite's official documentation](https://www.sqlite.org/json1.html).
/// In the case of json, some caveats must be taken:
///
/// - To insert json values you must use sqlite's `json` function in the query alongside named
/// parameters, and the provided parameter must be a valid json. That is:
///
/// ```sql
/// INSERT INTO SOME_TABLE
///     (some_field)
///     VALUES (json(:some_field))")
/// ```
///
/// ```ignore
/// let metadata = format!(r#"{{"some_inner_field": {some_field}, "some_other_inner_field": {some_other_field}}}"#);
/// ```
///
/// (Using raw string literals for the jsons is encouraged if possible)
///
/// - To get data from any of the json fields you can use the `json_extract` function (in some
/// cases you'll need to do some explicit type casting to help rusqlite figure out types):
///
/// ```sql
/// SELECT CAST(json_extract(some_json_col, '$.some_json_field') AS TEXT) from some_table
/// ```
///
/// - For some datatypes you'll need to do some manual serialization/deserialization. For example,
/// suppose one of your json fields is an array of digests. Then you'll need to
///     - Create the json with an array of strings representing the digests:
///
///     ```ignore
///     let some_array_field = some_array
///         .into_iter()
///         .map(array_elem_to_string)
///         .collect::<Vec<_>>()
///         .join(",");
///
///     Some(format!(
///         r#"{{
///             "some_array_field": [{some_array_field}]
///         }}"#
///     )),
///     ```
///
///     - When deserializing, handling the extra symbols (`[`, `]`, `,`, `"`). For that you can use
///     the `parse_json_array` function:
///
///     ```ignore
///         let some_array = parse_json_array(some_array_field)
///         .into_iter()
///         .map(parse_json_byte_str)
///         .collect::<Result<Vec<u8>, _>>()?;
///     ```

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

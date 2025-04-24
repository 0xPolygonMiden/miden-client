use rusqlite::{OptionalExtension, params};

use super::{connection::Connection, transaction::Transaction};

/// Checks if a table exists in the database.
pub fn table_exists(transaction: &Transaction, table_name: &str) -> rusqlite::Result<bool> {
    Ok(transaction
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = $1",
            params![table_name],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

/// Returns the schema version of the database.
pub fn schema_version(connection: &mut Connection) -> rusqlite::Result<usize> {
    connection
        .transaction()?
        .query_row("SELECT * FROM pragma_schema_version", [], |row| row.get(0))
}

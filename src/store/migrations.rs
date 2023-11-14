use lazy_static::lazy_static;

use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, M};

use super::StoreError;

// MIGRATIONS
// ================================================================================================

lazy_static! {
    static ref MIGRATIONS: Migrations<'static> =
        Migrations::new(vec![M::up(include_str!("store.sql")),]);
}

// PUBLIC FUNCTIONS
// ================================================================================================

pub fn update_to_latest(conn: &mut Connection) -> Result<(), StoreError> {
    MIGRATIONS
        .to_latest(conn)
        .map_err(StoreError::MigrationError)
}

pub fn _insert_mock_data(conn: &Connection) {
    conn.execute(
        "INSERT INTO accounts (id, nonce, status) VALUES (?1, 1234, 1)",
        params!["3972335011818762557"],
    )
    .unwrap();
}

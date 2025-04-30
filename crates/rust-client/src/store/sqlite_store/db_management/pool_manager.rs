use std::path::PathBuf;

use deadpool::{
    Runtime,
    managed::{Manager, Metrics, RecycleResult},
};
use rusqlite::{Connection, vtab::array};

use super::errors::SqliteStoreError;

deadpool::managed_reexports!(
    "miden-client-sqlite-store",
    SqlitePoolManager,
    deadpool::managed::Object<SqlitePoolManager>,
    rusqlite::Error,
    SqliteStoreError
);

const RUNTIME: Runtime = Runtime::Tokio1;

// POOL MANAGER
// ================================================================================================

/// `SQLite` connection pool manager
pub struct SqlitePoolManager {
    database_path: PathBuf,
}

/// `SQLite` connection pool manager
impl SqlitePoolManager {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }

    fn new_connection(&self) -> rusqlite::Result<Connection> {
        let conn = Connection::open(&self.database_path)?;

        // Feature used to support `IN` and `NOT IN` queries. We need to load
        // this module for every connection we create to the DB to support the
        // queries we want to run
        array::load_module(&conn)?;

        // Enable foreign key checks.
        conn.pragma_update(None, "foreign_keys", "ON")?;

        Ok(conn)
    }
}

impl Manager for SqlitePoolManager {
    type Type = deadpool_sync::SyncWrapper<Connection>;
    type Error = rusqlite::Error;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let conn = self.new_connection();
        deadpool_sync::SyncWrapper::new(RUNTIME, move || conn).await
    }

    async fn recycle(&self, _: &mut Self::Type, _: &Metrics) -> RecycleResult<Self::Error> {
        Ok(())
    }
}

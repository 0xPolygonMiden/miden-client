use std::{
    string::{String, ToString},
    sync::LazyLock,
    vec::Vec,
};

use miden_objects::crypto::hash::blake::{Blake3_160, Blake3Digest};
use rusqlite::{Connection, OptionalExtension, Result, ToSql, Transaction, params, types::FromSql};
use rusqlite_migration::{M, Migrations, SchemaVersion};

use super::errors::SqliteStoreError;

// MACROS
// ================================================================================================

/// Auxiliary macro which substitutes `$src` token by `$dst` expression.
#[macro_export]
macro_rules! subst {
    ($src:tt, $dst:expr_2021) => {
        $dst
    };
}

/// Generates a simple insert SQL statement with parameters for the provided table name and fields.
/// Supports optional conflict resolution (adding "| REPLACE" or "| IGNORE" at the end will generate
/// "OR REPLACE" and "OR IGNORE", correspondingly).
///
/// # Usage:
///
/// ```ignore
/// insert_sql!(users { id, first_name, last_name, age } | REPLACE);
/// ```
///
/// which generates:
/// ```sql
/// INSERT OR REPLACE INTO `users` (`id`, `first_name`, `last_name`, `age`) VALUES (?, ?, ?, ?)
/// ```
#[macro_export]
macro_rules! insert_sql {
    ($table:ident { $first_field:ident $(, $($field:ident),+)? $(,)? } $(| $on_conflict:expr)?) => {
        concat!(
            stringify!(INSERT $(OR $on_conflict)? INTO ),
            "`",
            stringify!($table),
            "` (`",
            stringify!($first_field),
            $($(concat!("`, `", stringify!($field))),+ ,)?
            "`) VALUES (",
            subst!($first_field, "?"),
            $($(subst!($field, ", ?")),+ ,)?
            ")"
        )
    };
}

// MIGRATIONS
// ================================================================================================

type Hash = Blake3Digest<20>;

const MIGRATION_SCRIPTS: [&str; 1] = [include_str!("../store.sql")];
static MIGRATION_HASHES: LazyLock<Vec<Hash>> = LazyLock::new(compute_migration_hashes);
static MIGRATIONS: LazyLock<Migrations> = LazyLock::new(prepare_migrations);

fn up(s: &'static str) -> M<'static> {
    M::up(s).foreign_key_check()
}

const DB_MIGRATION_HASH_FIELD: &str = "db-migration-hash";
const DB_SCHEMA_VERSION_FIELD: &str = "db-schema-version";

/// Applies the migrations to the database.
pub fn apply_migrations(conn: &mut Connection) -> Result<(), SqliteStoreError> {
    let version_before = MIGRATIONS.current_version(conn)?;

    if let SchemaVersion::Inside(ver) = version_before {
        if !table_exists(&conn.transaction()?, "settings")? {
            return Err(SqliteStoreError::MissingSettingsTable);
        }

        let last_schema_version: usize = get_settings_value(conn, DB_SCHEMA_VERSION_FIELD)?
            .ok_or_else(|| {
                SqliteStoreError::DatabaseError("Schema version not found".to_string())
            })?;

        let current_schema_version = schema_version(conn)?;

        if last_schema_version != current_schema_version {
            return Err(SqliteStoreError::SchemaVersionMismatch);
        }

        let expected_hash = &*MIGRATION_HASHES[ver.get() - 1];
        let actual_hash =
            hex::decode(get_settings_value::<String>(conn, DB_MIGRATION_HASH_FIELD)?.ok_or_else(
                || SqliteStoreError::DatabaseError("Migration hash not found".to_string()),
            )?)
            .map_err(|e| SqliteStoreError::HexDecodeError(e.to_string()))?;

        if actual_hash != expected_hash {
            return Err(SqliteStoreError::MigrationHashMismatch);
        }
    }

    MIGRATIONS.to_latest(conn)?;

    let version_after = MIGRATIONS.current_version(conn)?;

    if version_before != version_after {
        let new_hash = hex::encode(&*MIGRATION_HASHES[MIGRATION_HASHES.len() - 1]);
        set_settings_value(conn, DB_MIGRATION_HASH_FIELD, &new_hash)?;
    }

    let new_schema_version = schema_version(conn)?;
    set_settings_value(conn, DB_SCHEMA_VERSION_FIELD, &new_schema_version)?;

    Ok(())
}

fn prepare_migrations() -> Migrations<'static> {
    Migrations::new(MIGRATION_SCRIPTS.map(up).to_vec())
}

fn compute_migration_hashes() -> Vec<Hash> {
    let mut accumulator = Hash::default();
    MIGRATION_SCRIPTS
        .iter()
        .map(|sql| {
            let script_hash = Blake3_160::hash(preprocess_sql(sql).as_bytes());
            accumulator = Blake3_160::merge(&[accumulator, script_hash]);
            accumulator
        })
        .collect()
}

fn preprocess_sql(sql: &str) -> String {
    // TODO: We can also remove all comments here (need to analyze the SQL script in order to remove
    //       comments in string literals).
    remove_spaces(sql)
}

fn remove_spaces(str: &str) -> String {
    str.chars().filter(|chr| !chr.is_whitespace()).collect()
}

pub fn get_settings_value<T: FromSql>(conn: &mut Connection, name: &str) -> Result<Option<T>> {
    conn.transaction()?
        .query_row("SELECT value FROM settings WHERE name = $1", params![name], |row| row.get(0))
        .optional()
}

pub fn set_settings_value<T: ToSql>(conn: &Connection, name: &str, value: &T) -> Result<()> {
    let count =
        conn.execute(insert_sql!(settings { name, value } | REPLACE), params![name, value])?;

    debug_assert_eq!(count, 1);

    Ok(())
}

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

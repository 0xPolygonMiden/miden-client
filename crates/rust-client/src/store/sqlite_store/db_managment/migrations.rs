use std::{string::String, sync::LazyLock, vec::Vec};

use miden_objects::crypto::hash::blake::{Blake3_160, Blake3Digest};
use rusqlite_migration::{M, Migrations, SchemaVersion};

use super::connection::Connection;
use crate::store::{
    StoreError,
    sqlite_store::db_managment::{schema_version, settings::Settings},
};

type Hash = Blake3Digest<20>;

const MIGRATION_SCRIPTS: [&str; 1] = [include_str!("../store.sql")];
static MIGRATION_HASHES: LazyLock<Vec<Hash>> = LazyLock::new(compute_migration_hashes);
static MIGRATIONS: LazyLock<Migrations> = LazyLock::new(prepare_migrations);

fn up(s: &'static str) -> M<'static> {
    M::up(s).foreign_key_check()
}

const DB_MIGRATION_HASH_FIELD: &str = "db-migration-hash";
const DB_SCHEMA_VERSION_FIELD: &str = "db-schema-version";

pub fn apply_migrations(conn: &mut Connection) -> Result<(), StoreError> {
    let version_before = MIGRATIONS.current_version(conn).unwrap();

    if let SchemaVersion::Inside(ver) = version_before {
        if !Settings::exists(conn).unwrap() {
            panic!("No settings table in the database");
        }

        let last_schema_version: usize =
            Settings::get_value(conn, DB_SCHEMA_VERSION_FIELD).unwrap().unwrap();
        let current_schema_version = schema_version(conn).unwrap();

        if last_schema_version != current_schema_version {
            panic!("Schema version mismatch");
        }

        let expected_hash = &*MIGRATION_HASHES[ver.get() - 1];
        let actual_hash = hex::decode(
            Settings::get_value::<String>(conn, DB_MIGRATION_HASH_FIELD).unwrap().unwrap(),
        )
        .unwrap();

        if actual_hash != expected_hash {
            panic!("Migration hashes mismatch");
        }
    }

    MIGRATIONS.to_latest(conn).unwrap();

    let version_after = MIGRATIONS.current_version(conn).unwrap();

    if version_before != version_after {
        let new_hash = hex::encode(&*MIGRATION_HASHES[MIGRATION_HASHES.len() - 1]);
        Settings::set_value(conn, DB_MIGRATION_HASH_FIELD, &new_hash).unwrap();
    }

    // Run full database optimization. This will run indexes analysis for the query planner.
    // This will also increase the `schema_version` value.
    //
    // We should run full database optimization in following cases:
    // 1. Once schema was changed, especially new indexes were created.
    // 2. After restarting of the node, on first connection established.
    //
    // More info: https://www.sqlite.org/pragma.html#pragma_optimize
    conn.pragma_update(None, "optimize", "0x10002")?;

    let new_schema_version = schema_version(conn).unwrap();
    Settings::set_value(conn, DB_SCHEMA_VERSION_FIELD, &new_schema_version).unwrap();

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

#[test]
fn migrations_validate() {
    assert_eq!(MIGRATIONS.validate(), Ok(()));
}

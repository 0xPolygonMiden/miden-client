use rusqlite::{OptionalExtension, Result, ToSql, params, types::FromSql};

use super::{connection::Connection, utils::table_exists};

// UTILS
// ================================================================================================

/// Auxiliary macro which substitutes `$src` token by `$dst` expression.
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

// SETTINGS
// ================================================================================================

/// `SQLite` settings
pub struct Settings;

impl Settings {
    pub fn exists(conn: &mut Connection) -> Result<bool> {
        table_exists(&conn.transaction()?, "settings")
    }

    pub fn get_value<T: FromSql>(conn: &mut Connection, name: &str) -> Result<Option<T>> {
        conn.transaction()?
            .query_row("SELECT value FROM settings WHERE name = $1", params![name], |row| {
                row.get(0)
            })
            .optional()
    }

    pub fn set_value<T: ToSql>(conn: &Connection, name: &str, value: &T) -> Result<()> {
        let count =
            conn.execute(insert_sql!(settings { name, value } | REPLACE), params![name, value])?;

        debug_assert_eq!(count, 1);

        Ok(())
    }
}

use alloc::string::ToString;

use crate::store::StoreError;

// STORE ERROR
// ================================================================================================

impl From<rusqlite_migration::Error> for StoreError {
    fn from(value: rusqlite_migration::Error) -> Self {
        Self::DatabaseError(value.to_string())
    }
}
impl From<rusqlite::Error> for StoreError {
    fn from(value: rusqlite::Error) -> Self {
        match value {
            rusqlite::Error::FromSqlConversionFailure(..)
            | rusqlite::Error::IntegralValueOutOfRange(..)
            | rusqlite::Error::InvalidColumnIndex(_)
            | rusqlite::Error::InvalidColumnType(..) => Self::ParsingError(value.to_string()),
            rusqlite::Error::InvalidParameterName(_)
            | rusqlite::Error::InvalidColumnName(_)
            | rusqlite::Error::StatementChangedRows(_)
            | rusqlite::Error::ExecuteReturnedResults
            | rusqlite::Error::InvalidQuery
            | rusqlite::Error::MultipleStatement
            | rusqlite::Error::InvalidParameterCount(..)
            | rusqlite::Error::QueryReturnedNoRows => Self::QueryError(value.to_string()),
            _ => Self::DatabaseError(value.to_string()),
        }
    }
}

use core::fmt;

// CLIENT ERROR
// ================================================================================================

#[derive(Debug, PartialEq)]
pub enum ClientError {
    StoreError(StoreError),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::StoreError(err) => write!(f, "store error: {err}"),
        }
    }
}

impl From<StoreError> for ClientError {
    fn from(err: StoreError) -> Self {
        Self::StoreError(err)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ClientError {}

// STORE ERROR
// ================================================================================================

#[derive(Debug, PartialEq)]
pub enum StoreError {
    ConnectionError(rusqlite::Error),
    MigrationError(rusqlite_migration::Error),
    QueryError(rusqlite::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use StoreError::*;
        match self {
            ConnectionError(err) => write!(f, "failed to connect to the database: {err}"),
            MigrationError(err) => write!(f, "failed to update the database: {err}"),
            QueryError(err) => write!(f, "failed to retrieve data from the database: {err}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for StoreError {}

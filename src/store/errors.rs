use core::fmt;
use crypto::utils::DeserializationError;
use objects::{accounts::AccountId, Digest};

#[derive(Debug)]
pub enum StoreError {
    ConnectionError(rusqlite::Error),
    ColumnParsingError(rusqlite::Error),
    QueryError(rusqlite::Error),
    InputSerializationError(serde_json::Error),
    JsonDataDeserializationError(serde_json::Error),
    DataDeserializationError(DeserializationError),
    AccountDataNotFound(AccountId),
    AccountStorageNotFound(Digest),
    VaultDataNotFound(Digest),
    AccountCodeDataNotFound(Digest),
    InputNoteNotFound(Digest),
    MigrationError(rusqlite_migration::Error),
    TransactionError(rusqlite::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use StoreError::*;
        match self {
            ConnectionError(err) => write!(f, "failed to connect to the database: {err}"),
            MigrationError(err) => write!(f, "failed to update the database: {err}"),
            QueryError(err) => write!(f, "failed to retrieve data from the database: {err}"),
            TransactionError(err) => write!(f, "failed to instantiate a new transaction: {err}"),
            ColumnParsingError(err) => {
                write!(f, "failed to parse data retrieved from the database: {err}")
            }
            InputSerializationError(err) => {
                write!(f, "error trying to serialize inputs for the store: {err}")
            }
            JsonDataDeserializationError(err) => {
                write!(
                    f,
                    "error deserializing data from JSON from the store: {err}"
                )
            }
            DataDeserializationError(err) => {
                write!(f, "error deserializing data from the store: {err}")
            }
            AccountDataNotFound(account_id) => {
                write!(f, "Account data was not found for Account Id {account_id}")
            }
            InputNoteNotFound(hash) => write!(f, "input note with hash {} not found", hash),
            AccountStorageNotFound(root) => {
                write!(f, "account storage data with root {} not found", root)
            }
            VaultDataNotFound(root) => write!(f, "account vault data for root {} not found", root),
            AccountCodeDataNotFound(root) => {
                write!(f, "account code data with root {} not found", root)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for StoreError {}

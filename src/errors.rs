use core::fmt;
use crypto::utils::{DeserializationError, HexParseError};
use objects::{accounts::AccountId, AccountError, Digest, TransactionScriptError};
use tonic::{transport::Error as TransportError, Status as TonicStatus};

// CLIENT ERROR
// ================================================================================================

#[derive(Debug)]
pub enum ClientError {
    StoreError(StoreError),
    AccountError(AccountError),
    RpcApiError(RpcApiError),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::StoreError(err) => write!(f, "store error: {err}"),
            ClientError::AccountError(err) => write!(f, "account error: {err}"),
            ClientError::RpcApiError(err) => write!(f, "rpc api error: {err}"),
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

#[derive(Debug)]
pub enum StoreError {
    ConnectionError(rusqlite::Error),
    ColumnParsingError(rusqlite::Error),
    QueryError(rusqlite::Error),
    InputSerializationError(serde_json::Error),
    JsonDataDeserializationError(serde_json::Error),
    HexParseError(HexParseError),
    DataDeserializationError(DeserializationError),
    AccountError(AccountError),
    AccountDataNotFound(AccountId),
    AccountStorageNotFound(Digest),
    VaultDataNotFound(Digest),
    AccountCodeDataNotFound(Digest),
    InputNoteNotFound(Digest),
    MigrationError(rusqlite_migration::Error),
    TransactionError(rusqlite::Error),
    TransactionScriptError(TransactionScriptError),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use StoreError::*;
        match self {
            AccountError(err) => write!(f, "error instantiating Account: {err}"),
            ConnectionError(err) => write!(f, "failed to connect to the database: {err}"),
            MigrationError(err) => write!(f, "failed to update the database: {err}"),
            QueryError(err) => write!(f, "failed to retrieve data from the database: {err}"),
            ColumnParsingError(err) => {
                write!(f, "failed to parse data retrieved from the database: {err}")
            }
            InputSerializationError(err) => {
                write!(f, "error trying to serialize inputs for the store: {err}")
            }
            HexParseError(err) => {
                write!(f, "error parsing hex: {err}")
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
            TransactionError(err) => write!(f, "failed to instantiate a new transaction: {err}"),
            TransactionScriptError(err) => {
                write!(f, "error instantiating transaction script: {err}")
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

// API CLIENT ERROR
// ================================================================================================

#[derive(Debug)]
pub enum RpcApiError {
    ConnectionError(TransportError),
    RequestError(TonicStatus),
}

impl fmt::Display for RpcApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcApiError::ConnectionError(err) => {
                write!(f, "failed to connect to the API server: {err}")
            }
            RpcApiError::RequestError(err) => write!(f, "rpc request failed: {err}"),
        }
    }
}

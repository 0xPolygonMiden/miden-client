use core::fmt;
use crypto::utils::{DeserializationError, HexParseError};
use miden_tx::{TransactionExecutorError, TransactionProverError};
use objects::{accounts::AccountId, AccountError, Digest, NoteError, TransactionScriptError};
use tonic::{transport::Error as TransportError, Status as TonicStatus};

// CLIENT ERROR
// ================================================================================================

#[derive(Debug)]
pub enum ClientError {
    AccountError(AccountError),
    NoteError(NoteError),
    RpcApiError(RpcApiError),
    StoreError(StoreError),
    TransactionExecutorError(TransactionExecutorError),
    TransactionProverError(TransactionProverError),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::AccountError(err) => write!(f, "account error: {err}"),
            ClientError::NoteError(err) => write!(f, "note error: {err}"),
            ClientError::RpcApiError(err) => write!(f, "rpc api error: {err}"),
            ClientError::StoreError(err) => write!(f, "store error: {err}"),
            ClientError::TransactionExecutorError(err) => {
                write!(f, "transaction executor error: {err}")
            }
            ClientError::TransactionProverError(err) => {
                write!(f, "transaction prover error: {err}")
            }
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
    AccountCodeDataNotFound(Digest),
    AccountDataNotFound(AccountId),
    AccountError(AccountError),
    AccountStorageNotFound(Digest),
    ColumnParsingError(rusqlite::Error),
    ConnectionError(rusqlite::Error),
    DataDeserializationError(DeserializationError),
    HexParseError(HexParseError),
    InputNoteNotFound(Digest),
    InputSerializationError(serde_json::Error),
    JsonDataDeserializationError(serde_json::Error),
    MigrationError(rusqlite_migration::Error),
    NoteTagAlreadyTracked(u64),
    QueryError(rusqlite::Error),
    TransactionError(rusqlite::Error),
    TransactionScriptError(TransactionScriptError),
    VaultDataNotFound(Digest),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use StoreError::*;
        match self {
            AccountCodeDataNotFound(root) => {
                write!(f, "account code data with root {} not found", root)
            }
            AccountDataNotFound(account_id) => {
                write!(f, "Account data was not found for Account Id {account_id}")
            }
            AccountError(err) => write!(f, "error instantiating Account: {err}"),
            AccountStorageNotFound(root) => {
                write!(f, "account storage data with root {} not found", root)
            }
            ColumnParsingError(err) => {
                write!(f, "failed to parse data retrieved from the database: {err}")
            }
            ConnectionError(err) => write!(f, "failed to connect to the database: {err}"),
            DataDeserializationError(err) => {
                write!(f, "error deserializing data from the store: {err}")
            }
            HexParseError(err) => {
                write!(f, "error parsing hex: {err}")
            }
            InputNoteNotFound(hash) => write!(f, "input note with hash {} not found", hash),
            InputSerializationError(err) => {
                write!(f, "error trying to serialize inputs for the store: {err}")
            }
            JsonDataDeserializationError(err) => {
                write!(
                    f,
                    "error deserializing data from JSON from the store: {err}"
                )
            }
            MigrationError(err) => write!(f, "failed to update the database: {err}"),
            NoteTagAlreadyTracked(tag) => write!(f, "note tag {} is already being tracked", tag),
            QueryError(err) => write!(f, "failed to retrieve data from the database: {err}"),
            TransactionError(err) => write!(f, "failed to instantiate a new transaction: {err}"),
            TransactionScriptError(err) => {
                write!(f, "error instantiating transaction script: {err}")
            }
            VaultDataNotFound(root) => write!(f, "account vault data for root {} not found", root),
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

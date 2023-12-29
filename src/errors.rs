use core::fmt;
use crypto::merkle::MmrError;
use crypto::utils::DeserializationError;
use crypto::{dsa::rpo_falcon512::FalconError, utils::HexParseError};
use miden_node_proto::error::ParseError;
use miden_tx::{TransactionExecutorError, TransactionProverError};
use objects::AssetError;
use objects::{accounts::AccountId, AccountError, Digest, NoteError, TransactionScriptError};
use tonic::{transport::Error as TransportError, Status as TonicStatus};

// CLIENT ERROR
// ================================================================================================

#[derive(Debug)]
pub enum ClientError {
    AccountError(AccountError),
    AssetError(AssetError),
    AuthError(FalconError),
    RpcTypeConversionFailure(ParseError),
    NoteError(NoteError),
    RpcApiError(RpcApiError),
    StoreError(StoreError),
    TransactionExecutionError(TransactionExecutorError),
    TransactionProvingError(TransactionProverError),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::AccountError(err) => write!(f, "account error: {err}"),
            ClientError::AssetError(err) => write!(f, "asset error: {err}"),
            ClientError::AuthError(err) => write!(f, "account auth error: {err}"),
            ClientError::RpcTypeConversionFailure(err) => {
                write!(f, "failed to convert data: {err}")
            }
            ClientError::NoteError(err) => write!(f, "note error: {err}"),
            ClientError::RpcApiError(err) => write!(f, "rpc api error: {err}"),
            ClientError::StoreError(err) => write!(f, "store error: {err}"),
            ClientError::TransactionExecutionError(err) => {
                write!(f, "transaction executor error: {err}")
            }
            ClientError::TransactionProvingError(err) => {
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
    AccountHashMismatch(AccountId),
    AccountStorageNotFound(Digest),
    ChainMmrNodeNotFound(u64),
    ColumnParsingError(rusqlite::Error),
    ConnectionError(rusqlite::Error),
    DataDeserializationError(DeserializationError),
    HexParseError(HexParseError),
    InputNoteNotFound(Digest),
    InputSerializationError(serde_json::Error),
    JsonDataDeserializationError(serde_json::Error),
    MigrationError(rusqlite_migration::Error),
    MmrError(MmrError),
    NoteTagAlreadyTracked(u64),
    QueryError(rusqlite::Error),
    TransactionError(rusqlite::Error),
    BlockHeaderNotFound(u32),
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
            AccountHashMismatch(account_id) => {
                write!(f, "account hash mismatch for account {account_id}")
            }
            AccountStorageNotFound(root) => {
                write!(f, "account storage data with root {} not found", root)
            }
            ColumnParsingError(err) => {
                write!(f, "failed to parse data retrieved from the database: {err}")
            }
            ChainMmrNodeNotFound(node_index) => {
                write!(f, "chain mmr node at index {} not found", node_index)
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
            MmrError(err) => write!(f, "error constructing mmr: {err}"),
            NoteTagAlreadyTracked(tag) => write!(f, "note tag {} is already being tracked", tag),
            QueryError(err) => write!(f, "failed to retrieve data from the database: {err}"),
            TransactionError(err) => write!(f, "failed to instantiate a new transaction: {err}"),
            TransactionScriptError(err) => {
                write!(f, "error instantiating transaction script: {err}")
            }
            VaultDataNotFound(root) => write!(f, "account vault data for root {} not found", root),
            BlockHeaderNotFound(block_num) => {
                write!(f, "block header for block {} not found", block_num)
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

use core::fmt;
use crypto::{
    dsa::rpo_falcon512::FalconError,
    merkle::MmrError,
    utils::{DeserializationError, HexParseError},
};
use miden_node_proto::error::ParseError;
use miden_tx::{TransactionExecutorError, TransactionProverError};
use objects::{
    accounts::AccountId, notes::NoteId, AccountError, AssetVaultError, Digest, NoteError,
    TransactionScriptError,
};
use tonic::{transport::Error as TransportError, Status as TonicStatus};

// CLIENT ERROR
// ================================================================================================

#[derive(Debug)]
pub enum ClientError {
    AccountError(AccountError),
    AuthError(FalconError),
    NoteError(NoteError),
    RpcApiError(RpcApiError),
    RpcExpectedFieldMissingFailure(String),
    RpcTypeConversionFailure(ParseError),
    StoreError(StoreError),
    TransactionExecutionError(TransactionExecutorError),
    TransactionProvingError(TransactionProverError),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::AccountError(err) => write!(f, "account error: {err}"),
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
            ClientError::RpcExpectedFieldMissingFailure(err) => {
                write!(f, "rpc api reponse missing an expected field: {err}")
            }
        }
    }
}

impl From<AccountError> for ClientError {
    fn from(err: AccountError) -> Self {
        Self::AccountError(err)
    }
}

impl From<NoteError> for ClientError {
    fn from(err: NoteError) -> Self {
        Self::NoteError(err)
    }
}

impl From<ParseError> for ClientError {
    fn from(err: ParseError) -> Self {
        Self::RpcTypeConversionFailure(err)
    }
}

impl From<StoreError> for ClientError {
    fn from(err: StoreError) -> Self {
        Self::StoreError(err)
    }
}

impl From<TransactionExecutorError> for ClientError {
    fn from(err: TransactionExecutorError) -> Self {
        Self::TransactionExecutionError(err)
    }
}

impl From<TransactionProverError> for ClientError {
    fn from(err: TransactionProverError) -> Self {
        Self::TransactionProvingError(err)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ClientError {}

// STORE ERROR
// ================================================================================================

#[derive(Debug)]
pub enum StoreError {
    AssetVaultError(AssetVaultError),
    AccountCodeDataNotFound(Digest),
    AccountDataNotFound(AccountId),
    AccountError(AccountError),
    AccountHashMismatch(AccountId),
    AccountStorageNotFound(Digest),
    BlockHeaderNotFound(u32),
    ChainMmrNodeNotFound(u64),
    ColumnParsingError(rusqlite::Error),
    ConnectionError(rusqlite::Error),
    DataDeserializationError(DeserializationError),
    HexParseError(HexParseError),
    InputNoteNotFound(NoteId),
    InputSerializationError(serde_json::Error),
    JsonDataDeserializationError(serde_json::Error),
    MigrationError(rusqlite_migration::Error),
    MmrError(MmrError),
    NoteTagAlreadyTracked(u64),
    QueryError(rusqlite::Error),
    RpcTypeConversionFailure(ParseError),
    TransactionError(rusqlite::Error),
    TransactionScriptError(TransactionScriptError),
    VaultDataNotFound(Digest),
}

impl From<AssetVaultError> for StoreError {
    fn from(value: AssetVaultError) -> Self {
        StoreError::AssetVaultError(value)
    }
}

impl From<AccountError> for StoreError {
    fn from(value: AccountError) -> Self {
        StoreError::AccountError(value)
    }
}

impl From<DeserializationError> for StoreError {
    fn from(value: DeserializationError) -> Self {
        StoreError::DataDeserializationError(value)
    }
}

impl From<HexParseError> for StoreError {
    fn from(value: HexParseError) -> Self {
        StoreError::HexParseError(value)
    }
}

impl From<MmrError> for StoreError {
    fn from(value: MmrError) -> Self {
        StoreError::MmrError(value)
    }
}

impl From<TransactionScriptError> for StoreError {
    fn from(value: TransactionScriptError) -> Self {
        StoreError::TransactionScriptError(value)
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use StoreError::*;
        match self {
            AssetVaultError(err) => {
                write!(f, "asset vault with root {} not found", err)
            }
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
            BlockHeaderNotFound(block_number) => {
                write!(f, "block header for block {} not found", block_number)
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
            InputNoteNotFound(note_id) => {
                write!(f, "input note with note id {} not found", note_id.inner())
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
            MigrationError(err) => write!(f, "failed to update the database: {err}"),
            MmrError(err) => write!(f, "error constructing mmr: {err}"),
            NoteTagAlreadyTracked(tag) => write!(f, "note tag {} is already being tracked", tag),
            QueryError(err) => write!(f, "failed to retrieve data from the database: {err}"),
            TransactionError(err) => write!(f, "failed to instantiate a new transaction: {err}"),
            TransactionScriptError(err) => {
                write!(f, "error instantiating transaction script: {err}")
            }
            VaultDataNotFound(root) => write!(f, "account vault data for root {} not found", root),
            RpcTypeConversionFailure(err) => write!(f, "failed to convert data: {err}"),
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

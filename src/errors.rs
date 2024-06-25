use core::fmt;

use miden_objects::{
    accounts::AccountId,
    crypto::merkle::{MmrError},
    notes::NoteId,
    AccountError, AssetError, AssetVaultError, Digest, NoteError, TransactionScriptError, Word,
};
use miden_tx::{
    utils::{DeserializationError, HexParseError},
    DataStoreError, TransactionExecutorError, TransactionProverError,
};

// CLIENT ERROR
// ================================================================================================

#[derive(Debug)]
pub enum ClientError {
    AccountError(AccountError),
    AssetError(AssetError),
    DataDeserializationError(DeserializationError),
    HexParseError(HexParseError),
    ImportNewAccountWithoutSeed,
    MissingOutputNotes(Vec<NoteId>),
    NoteError(NoteError),
    NoteImportError(String),
    NoteRecordError(String),
    NoConsumableNoteForAccount(AccountId),
    NodeRpcClientError(RpcError),
    ScreenerError(NoteScreenerError),
    StoreError(StoreError),
    TransactionExecutorError(TransactionExecutorError),
    TransactionProvingError(TransactionProverError),
    ExistenceVerificationError(NoteId),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::AccountError(err) => write!(f, "account error: {err}"),
            ClientError::DataDeserializationError(err) => {
                write!(f, "data deserialization error: {err}")
            },
            ClientError::AssetError(err) => write!(f, "asset error: {err}"),
            ClientError::HexParseError(err) => write!(f, "error turning array to Digest: {err}"),
            ClientError::ImportNewAccountWithoutSeed => write!(
                f,
                "import account error: can't import a new account without its initial seed"
            ),
            ClientError::MissingOutputNotes(note_ids) => {
                write!(
                    f,
                    "transaction error: The transaction did not produce the expected notes corresponding to Note IDs: {}",
                    note_ids.iter().map(|&id| id.to_hex()).collect::<Vec<_>>().join(", ")
                )
            },
            ClientError::NoConsumableNoteForAccount(account_id) => {
                write!(f, "No consumable note for account ID {}", account_id)
            },
            ClientError::NoteError(err) => write!(f, "note error: {err}"),
            ClientError::NoteImportError(err) => write!(f, "error importing note: {err}"),
            ClientError::NoteRecordError(err) => write!(f, "note record error: {err}"),
            ClientError::NodeRpcClientError(err) => write!(f, "rpc api error: {err}"),
            ClientError::ScreenerError(err) => write!(f, "note screener error: {err}"),
            ClientError::StoreError(err) => write!(f, "store error: {err}"),
            ClientError::TransactionExecutorError(err) => {
                write!(f, "transaction executor error: {err}")
            },
            ClientError::TransactionProvingError(err) => {
                write!(f, "transaction prover error: {err}")
            },
            ClientError::ExistenceVerificationError(note_id) => {
                write!(f, "The note with ID {note_id} doesn't exist in the chain")
            },
        }
    }
}

// CONVERSIONS
// ================================================================================================

impl From<AccountError> for ClientError {
    fn from(err: AccountError) -> Self {
        Self::AccountError(err)
    }
}

impl From<DeserializationError> for ClientError {
    fn from(err: DeserializationError) -> Self {
        Self::DataDeserializationError(err)
    }
}

impl From<HexParseError> for ClientError {
    fn from(err: HexParseError) -> Self {
        Self::HexParseError(err)
    }
}

impl From<NoteError> for ClientError {
    fn from(err: NoteError) -> Self {
        Self::NoteError(err)
    }
}

impl From<RpcError> for ClientError {
    fn from(err: RpcError) -> Self {
        Self::NodeRpcClientError(err)
    }
}

impl From<StoreError> for ClientError {
    fn from(err: StoreError) -> Self {
        Self::StoreError(err)
    }
}

impl From<TransactionExecutorError> for ClientError {
    fn from(err: TransactionExecutorError) -> Self {
        Self::TransactionExecutorError(err)
    }
}

impl From<TransactionProverError> for ClientError {
    fn from(err: TransactionProverError) -> Self {
        Self::TransactionProvingError(err)
    }
}

impl From<NoteScreenerError> for ClientError {
    fn from(err: NoteScreenerError) -> Self {
        Self::ScreenerError(err)
    }
}

#[cfg(feature = "sqlite")]
impl From<rusqlite::Error> for ClientError {
    fn from(err: rusqlite::Error) -> Self {
        Self::StoreError(StoreError::from(err))
    }
}

impl From<ClientError> for String {
    fn from(err: ClientError) -> String {
        err.to_string()
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
    AccountKeyNotFound(Word),
    AccountStorageNotFound(Digest),
    BlockHeaderNotFound(u32),
    ChainMmrNodeNotFound(u64),
    DatabaseError(String),
    DataDeserializationError(DeserializationError),
    HexParseError(HexParseError),
    NoteNotFound(NoteId),
    InputSerializationError(serde_json::Error),
    JsonDataDeserializationError(serde_json::Error),
    MmrError(MmrError),
    NoteInclusionProofError(NoteError),
    NoteTagAlreadyTracked(u64),
    ParsingError(String),
    QueryError(String),
    RpcConversionError(ConversionError),
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

#[cfg(feature = "sqlite")]
impl From<rusqlite_migration::Error> for StoreError {
    fn from(value: rusqlite_migration::Error) -> Self {
        StoreError::DatabaseError(value.to_string())
    }
}

#[cfg(feature = "sqlite")]
impl From<rusqlite::Error> for StoreError {
    fn from(value: rusqlite::Error) -> Self {
        match value {
            rusqlite::Error::FromSqlConversionFailure(..)
            | rusqlite::Error::IntegralValueOutOfRange(..)
            | rusqlite::Error::InvalidColumnIndex(_)
            | rusqlite::Error::InvalidColumnType(..) => StoreError::ParsingError(value.to_string()),
            rusqlite::Error::InvalidParameterName(_)
            | rusqlite::Error::InvalidColumnName(_)
            | rusqlite::Error::StatementChangedRows(_)
            | rusqlite::Error::ExecuteReturnedResults
            | rusqlite::Error::InvalidQuery
            | rusqlite::Error::MultipleStatement
            | rusqlite::Error::InvalidParameterCount(..)
            | rusqlite::Error::QueryReturnedNoRows => StoreError::QueryError(value.to_string()),
            _ => StoreError::DatabaseError(value.to_string()),
        }
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

impl From<NoteError> for StoreError {
    fn from(value: NoteError) -> Self {
        StoreError::NoteInclusionProofError(value)
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
            },
            AccountCodeDataNotFound(root) => {
                write!(f, "account code data with root {} not found", root)
            },
            AccountDataNotFound(account_id) => {
                write!(f, "Account data was not found for Account Id {account_id}")
            },
            AccountError(err) => write!(f, "error instantiating Account: {err}"),
            AccountHashMismatch(account_id) => {
                write!(f, "account hash mismatch for account {account_id}")
            },
            AccountKeyNotFound(pub_key) => {
                write!(f, "error: Public Key {} not found", Digest::from(pub_key))
            },
            AccountStorageNotFound(root) => {
                write!(f, "account storage data with root {} not found", root)
            },
            BlockHeaderNotFound(block_number) => {
                write!(f, "block header for block {} not found", block_number)
            },
            ChainMmrNodeNotFound(node_index) => {
                write!(f, "chain mmr node at index {} not found", node_index)
            },
            DatabaseError(err) => write!(f, "database-related non-query error: {err}"),
            DataDeserializationError(err) => {
                write!(f, "error deserializing data from the store: {err}")
            },
            HexParseError(err) => {
                write!(f, "error parsing hex: {err}")
            },
            NoteNotFound(note_id) => {
                write!(f, "note with note id {} not found", note_id.inner())
            },
            InputSerializationError(err) => {
                write!(f, "error trying to serialize inputs for the store: {err}")
            },
            JsonDataDeserializationError(err) => {
                write!(f, "error deserializing data from JSON from the store: {err}")
            },
            MmrError(err) => write!(f, "error constructing mmr: {err}"),
            NoteTagAlreadyTracked(tag) => write!(f, "note tag {} is already being tracked", tag),
            NoteInclusionProofError(error) => {
                write!(f, "inclusion proof creation error: {}", error)
            },
            ParsingError(err) => {
                write!(f, "failed to parse data retrieved from the database: {err}")
            },
            QueryError(err) => write!(f, "failed to retrieve data from the database: {err}"),
            TransactionScriptError(err) => {
                write!(f, "error instantiating transaction script: {err}")
            },
            VaultDataNotFound(root) => write!(f, "account vault data for root {} not found", root),
            RpcConversionError(err) => write!(f, "failed to convert data: {err}"),
        }
    }
}

impl From<StoreError> for DataStoreError {
    fn from(value: StoreError) -> Self {
        match value {
            StoreError::AccountDataNotFound(account_id) => {
                DataStoreError::AccountNotFound(account_id)
            },
            StoreError::BlockHeaderNotFound(block_num) => DataStoreError::BlockNotFound(block_num),
            StoreError::NoteNotFound(note_id) => DataStoreError::NoteNotFound(note_id),
            err => DataStoreError::InternalError(err.to_string()),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for StoreError {}

// API CLIENT ERROR
// ================================================================================================

#[derive(Debug)]
pub enum RpcError {
    ConnectionError(String),
    ConversionFailure(String),
    DeserializationError(DeserializationError),
    ExpectedFieldMissing(String),
    InvalidAccountReceived(String),
    NoteError(NoteError),
    RequestError(String, String),
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::ConnectionError(err) => {
                write!(f, "failed to connect to the API server: {err}")
            },
            RpcError::ConversionFailure(err) => {
                write!(f, "failed to convert RPC data: {err}")
            },
            RpcError::DeserializationError(err) => {
                write!(f, "failed to deserialize RPC data: {err}")
            },
            RpcError::ExpectedFieldMissing(err) => {
                write!(f, "rpc API response missing an expected field: {err}")
            },
            RpcError::InvalidAccountReceived(account_error) => {
                write!(f, "rpc API response contained an invalid account: {account_error}")
            },
            RpcError::NoteError(err) => {
                write!(f, "rpc API note failed to validate: {err}")
            },
            RpcError::RequestError(endpoint, err) => {
                write!(f, "rpc request failed for {endpoint}: {err}")
            },
        }
    }
}

impl From<AccountError> for RpcError {
    fn from(err: AccountError) -> Self {
        Self::InvalidAccountReceived(err.to_string())
    }
}

impl From<DeserializationError> for RpcError {
    fn from(err: DeserializationError) -> Self {
        Self::DeserializationError(err)
    }
}

impl From<NoteError> for RpcError {
    fn from(err: NoteError) -> Self {
        Self::NoteError(err)
    }
}

impl From<ConversionError> for RpcError {
    fn from(err: ConversionError) -> Self {
        Self::ConversionFailure(err.to_string())
    }
}

// NOTE SCREENER ERROR
// ================================================================================================

/// Error when screening notes to check relevance to a client
#[derive(Debug)]
pub enum NoteScreenerError {
    InvalidNoteInputsError(InvalidNoteInputsError),
    StoreError(StoreError),
}

impl From<InvalidNoteInputsError> for NoteScreenerError {
    fn from(error: InvalidNoteInputsError) -> Self {
        Self::InvalidNoteInputsError(error)
    }
}

impl From<StoreError> for NoteScreenerError {
    fn from(error: StoreError) -> Self {
        Self::StoreError(error)
    }
}

impl fmt::Display for NoteScreenerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteScreenerError::InvalidNoteInputsError(note_inputs_err) => {
                write!(f, "error while processing note inputs: {note_inputs_err}")
            },
            NoteScreenerError::StoreError(store_error) => {
                write!(f, "error while fetching data from the store: {store_error}")
            },
        }
    }
}

#[derive(Debug)]
pub enum InvalidNoteInputsError {
    AccountError(NoteId, AccountError),
    AssetError(NoteId, AssetError),
    NumInputsError(NoteId, usize),
    BlockNumberError(NoteId, u64),
}

impl fmt::Display for InvalidNoteInputsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidNoteInputsError::AccountError(note_id, account_error) => {
                write!(f, "account error for note with ID {}: {account_error}", note_id.to_hex())
            },
            InvalidNoteInputsError::AssetError(note_id, asset_error) => {
                write!(f, "asset error for note with ID {}: {asset_error}", note_id.to_hex())
            },
            InvalidNoteInputsError::NumInputsError(note_id, expected_num_inputs) => {
                write!(
                    f,
                    "expected {expected_num_inputs} note inputs for note with ID {}",
                    note_id.to_hex()
                )
            },
            InvalidNoteInputsError::BlockNumberError(note_id, read_height) => {
                write!(
                    f,
                    "note input representing block with value {read_height} for note with ID {}",
                    note_id.to_hex()
                )
            },
        }
    }
}

// ID PREFIX FETCH ERROR
// ================================================================================================

/// Error when Looking for a specific ID from a partial ID
#[derive(Debug, Eq, PartialEq)]
pub enum IdPrefixFetchError {
    NoMatch(String),
    MultipleMatches(String),
}

impl fmt::Display for IdPrefixFetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdPrefixFetchError::NoMatch(id) => {
                write!(f, "No matches were found with the {id}.")
            },
            IdPrefixFetchError::MultipleMatches(id) => {
                write!(
                    f,
                    "Found more than one element for the provided {id} and only one match is expected."
                )
            },
        }
    }
}

// CONVERSION ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ConversionError {
    NotAValidFelt,
    NoteTypeError(NoteError),
    MissingFieldInProtobufRepresentation {
        entity: &'static str,
        field_name: &'static str,
    },
}

impl core::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionError::NotAValidFelt => write!(f, "Value is not in the range 0..MODULUS"),
            ConversionError::NoteTypeError(err) => write!(f, "Invalid note type value: {}", err),
            ConversionError::MissingFieldInProtobufRepresentation { entity, field_name } => write!(
                f,
                "Field `{}` required to be filled in protobuf representation of {}",
                field_name, entity
            ),
        }
    }
}

impl Eq for ConversionError {}

impl From<NoteError> for ConversionError {
    fn from(error: NoteError) -> Self {
        ConversionError::NoteTypeError(error)
    }
}

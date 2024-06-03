use core::fmt;

use miden_objects::{accounts::AccountId, notes::NoteId, AccountError, AssetError, NoteError};
use miden_tx::{
    utils::{DeserializationError, HexParseError},
    TransactionExecutorError, TransactionProverError,
};

use crate::transactions::transaction_request::TransactionRequestError;
use crate::{client::NoteScreenerError, rpc::RpcError, store::StoreError};

// CLIENT ERROR
// ================================================================================================

#[derive(Debug)]
pub enum ClientError {
    AccountError(AccountError),
    AssetError(AssetError),
    DataDeserializationError(DeserializationError),
    ExistenceVerificationError(NoteId),
    HexParseError(HexParseError),
    ImportNewAccountWithoutSeed,
    MissingOutputNotes(Vec<NoteId>),
    NoteError(NoteError),
    NoteImportError(String),
    NoteRecordError(String),
    NoConsumableNoteForAccount(AccountId),
    NodeError(RpcError),
    NoteScreenerError(NoteScreenerError),
    StoreError(StoreError),
    TransactionExecutorError(TransactionExecutorError),
    TransactionProvingError(TransactionProverError),
    TransactionRequestError(TransactionRequestError),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::AccountError(err) => write!(f, "account error: {err}"),
            ClientError::AssetError(err) => write!(f, "asset error: {err}"),
            ClientError::DataDeserializationError(err) => {
                write!(f, "data deserialization error: {err}")
            },
            ClientError::ExistenceVerificationError(note_id) => {
                write!(f, "The note with ID {note_id} doesn't exist in the chain")
            },
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
            ClientError::NodeError(err) => write!(f, "rpc api error: {err}"),
            ClientError::NoteScreenerError(err) => write!(f, "note screener error: {err}"),
            ClientError::StoreError(err) => write!(f, "store error: {err}"),
            ClientError::TransactionExecutorError(err) => {
                write!(f, "transaction executor error: {err}")
            },
            ClientError::TransactionProvingError(err) => {
                write!(f, "transaction prover error: {err}")
            },
            ClientError::TransactionRequestError(err) => {
                write!(f, "transaction request error: {err}")
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
        Self::NodeError(err)
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
        Self::NoteScreenerError(err)
    }
}

impl From<TransactionRequestError> for ClientError {
    fn from(err: TransactionRequestError) -> Self {
        Self::TransactionRequestError(err)
    }
}

impl From<ClientError> for String {
    fn from(err: ClientError) -> String {
        err.to_string()
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ClientError {}

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

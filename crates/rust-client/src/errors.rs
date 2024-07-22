use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use miden_objects::{accounts::AccountId, notes::NoteId, AccountError, AssetError, NoteError};
use miden_tx::{
    utils::{DeserializationError, HexParseError},
    TransactionExecutorError, TransactionProverError,
};

use crate::{
    notes::NoteScreenerError,
    rpc::RpcError,
    store::StoreError,
    transactions::{
        request::TransactionRequestError, script_builder::TransactionScriptBuilderError,
    },
};

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
    RpcError(RpcError),
    NoteScreenerError(NoteScreenerError),
    StoreError(StoreError),
    TransactionExecutorError(TransactionExecutorError),
    TransactionProvingError(TransactionProverError),
    TransactionRequestError(TransactionRequestError),
    TransactionScriptBuilderError(TransactionScriptBuilderError),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::AccountError(err) => write!(f, "Account error: {err}"),
            ClientError::AssetError(err) => write!(f, "Asset error: {err}"),
            ClientError::DataDeserializationError(err) => {
                write!(f, "Data deserialization error: {err}")
            },
            ClientError::ExistenceVerificationError(note_id) => {
                write!(f, "The note with ID {note_id} doesn't exist in the chain")
            },
            ClientError::HexParseError(err) => write!(f, "Error turning array to Digest: {err}"),
            ClientError::ImportNewAccountWithoutSeed => write!(
                f,
                "Import account error: can't import a new account without its initial seed"
            ),
            ClientError::MissingOutputNotes(note_ids) => {
                write!(
                    f,
                    "Transaction error: The transaction did not produce the expected notes corresponding to Note IDs: {}",
                    note_ids.iter().map(|&id| id.to_hex()).collect::<Vec<_>>().join(", ")
                )
            },
            ClientError::NoConsumableNoteForAccount(account_id) => {
                write!(f, "No consumable note for account ID {}", account_id)
            },
            ClientError::NoteError(err) => write!(f, "Note error: {err}"),
            ClientError::NoteImportError(err) => write!(f, "Error importing note: {err}"),
            ClientError::NoteRecordError(err) => write!(f, "Note record error: {err}"),
            ClientError::RpcError(err) => write!(f, "Rpc api error: {err}"),
            ClientError::NoteScreenerError(err) => write!(f, "Note screener error: {err}"),
            ClientError::StoreError(err) => write!(f, "Store error: {err}"),
            ClientError::TransactionExecutorError(err) => {
                write!(f, "Transaction executor error: {err}")
            },
            ClientError::TransactionProvingError(err) => {
                write!(f, "Transaction prover error: {err}")
            },
            ClientError::TransactionRequestError(err) => {
                write!(f, "Transaction request error: {err}")
            },
            ClientError::TransactionScriptBuilderError(err) => {
                write!(f, "Transaction script builder error: {err}")
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
        Self::RpcError(err)
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

impl From<TransactionScriptBuilderError> for ClientError {
    fn from(err: TransactionScriptBuilderError) -> Self {
        Self::TransactionScriptBuilderError(err)
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

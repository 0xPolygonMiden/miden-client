use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_lib::account::interface::AccountInterfaceError;
use miden_objects::{
    AccountError, AssetError, Digest, NoteError, TransactionScriptError, account::AccountId,
    crypto::merkle::MerkleError, note::NoteId,
};
// RE-EXPORTS
// ================================================================================================
pub use miden_tx::AuthenticationError;
use miden_tx::{
    TransactionExecutorError, TransactionProverError,
    utils::{DeserializationError, HexParseError},
};
use thiserror::Error;

use crate::{
    note::NoteScreenerError,
    rpc::RpcError,
    store::{NoteRecordError, StoreError},
    transaction::TransactionRequestError,
};

// CLIENT ERROR
// ================================================================================================

/// Errors generated by the client.
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("account with id {0} is already being tracked")]
    AccountAlreadyTracked(AccountId),
    #[error("account error")]
    AccountError(#[from] AccountError),
    #[error("account with id {0} is locked")]
    AccountLocked(AccountId),
    #[error("network account commitment {0} doesn't match the imported account commitment")]
    AccountCommitmentMismatch(Digest),
    #[error("account with id {0} is private")]
    AccountIsPrivate(AccountId),
    #[error("account nonce is too low to import")]
    AccountNonceTooLow,
    #[error("asset error")]
    AssetError(#[source] AssetError),
    #[error("account data wasn't found for account id {0}")]
    AccountDataNotFound(AccountId),
    #[error("data deserialization error")]
    DataDeserializationError(#[from] DeserializationError),
    #[error("note with id {0} not found on chain")]
    NoteNotFoundOnChain(NoteId),
    #[error("error parsing hex")]
    HexParseError(#[from] HexParseError),
    #[error("can't add new account without seed")]
    AddNewAccountWithoutSeed,
    #[error("error with merkle path")]
    MerkleError(#[from] MerkleError),
    #[error("the transaction didn't produce the expected notes corresponding to note ids")]
    MissingOutputNotes(Vec<NoteId>),
    #[error("note error")]
    NoteError(#[from] NoteError),
    #[error("note import error: {0}")]
    NoteImportError(String),
    #[error("note record error")]
    NoteRecordError(#[from] NoteRecordError),
    #[error("no consumable note for account {0}")]
    NoConsumableNoteForAccount(AccountId),
    #[error("rpc api error")]
    RpcError(#[from] RpcError),
    #[error("recency condition error")]
    RecencyConditionError(String),
    #[error("note screener error")]
    NoteScreenerError(#[from] NoteScreenerError),
    #[error("store error")]
    StoreError(#[from] StoreError),
    #[error("transaction executor error: {0}")]
    TransactionExecutorError(#[from] TransactionExecutorError),
    #[error("transaction prover error")]
    TransactionProvingError(#[from] TransactionProverError),
    #[error("transaction request error")]
    TransactionRequestError(#[from] TransactionRequestError),
    #[error("transaction script builder error")]
    AccountInterfaceError(#[from] AccountInterfaceError),
    #[error("transaction script error")]
    TransactionScriptError(#[source] TransactionScriptError),
    #[error("client initialization error: {0}")]
    ClientInitializationError(String),
}

// CONVERSIONS
// ================================================================================================

impl From<ClientError> for String {
    fn from(err: ClientError) -> String {
        err.to_string()
    }
}

// ID PREFIX FETCH ERROR
// ================================================================================================

/// Error when Looking for a specific ID from a partial ID.
#[derive(Debug, Error)]
pub enum IdPrefixFetchError {
    /// No matches were found for the ID prefix.
    #[error("no matches were found with the {0}")]
    NoMatch(String),
    /// Multiple entities matched with the ID prefix.
    #[error("found more than one element for the provided {0} and only one match is expected")]
    MultipleMatches(String),
}

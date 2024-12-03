use alloc::string::{String, ToString};

use miden_objects::{
    accounts::AccountId,
    crypto::merkle::MmrError,
    notes::NoteId,
    utils::{DeserializationError, HexParseError},
    AccountError, AssetVaultError, Digest, NoteError, TransactionScriptError,
};
use miden_tx::DataStoreError;
use thiserror::Error;

use super::note_record::NoteRecordError;

// STORE ERROR
// ================================================================================================

/// Errors generated from the store.
#[derive(Debug, Error)]
#[allow(clippy::large_enum_variant)]
pub enum StoreError {
    #[error("asset vault error")]
    AssetVaultError(#[source] AssetVaultError),
    #[error("account code data with root {0} not found")]
    AccountCodeDataNotFound(Digest),
    #[error("account data was not found for account id {0}")]
    AccountDataNotFound(AccountId),
    #[error("account error: {0}")]
    AccountError(AccountError),
    #[error("account hash {0} already exists")]
    AccountHashAlreadyExists(Digest),
    #[error("account hash mismatch for account {0}")]
    AccountHashMismatch(AccountId),
    #[error("public key {0} not found")]
    AccountKeyNotFound(String),
    #[error("account storage data with root {0} not found")]
    AccountStorageNotFound(Digest),
    #[error("block header for block {0} not found")]
    BlockHeaderNotFound(u32),
    #[error("chain mmr node at index {0} not found")]
    ChainMmrNodeNotFound(u64),
    #[error("error deserializing data from the store")]
    DataDeserializationError(#[source] DeserializationError),
    #[error("database-related non-query error: {0}")]
    DatabaseError(String),
    #[error("error parsing hex: {0}")]
    //TODO: use source in this error when possible
    HexParseError(HexParseError),
    #[error("note with id {0} not found")]
    NoteNotFound(NoteId),
    #[error("note record error")]
    NoteRecordError(#[source] NoteRecordError),
    #[error("error constructing mmr: {0}")]
    //TODO: use source in this error when possible
    MmrError(MmrError),
    #[error("inclusion proof creation error")]
    NoteInclusionProofError(#[source] NoteError),
    #[error("note tag {0} is already being tracked")]
    NoteTagAlreadyTracked(u64),
    #[error("failed to parse data retrieved from the database: {0}")]
    ParsingError(String),
    #[error("failed to retrieve data from the database: {0}")]
    QueryError(String),
    #[error("error instantiating transaction script")]
    TransactionScriptError(#[source] TransactionScriptError),
    #[error("account vault data for root {0} not found")]
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

impl From<NoteRecordError> for StoreError {
    fn from(value: NoteRecordError) -> Self {
        StoreError::NoteRecordError(value)
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

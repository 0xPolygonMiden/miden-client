use alloc::string::{String, ToString};
use core::fmt;

use miden_objects::{
    accounts::AccountId,
    crypto::merkle::MmrError,
    notes::NoteId,
    utils::{DeserializationError, HexParseError},
    AccountError, AssetVaultError, Digest, NoteError, TransactionScriptError, Word,
};
use miden_tx::DataStoreError;

// STORE ERROR
// ================================================================================================

/// Errors generated from the store.
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
    DataDeserializationError(DeserializationError),
    DatabaseError(String),
    HexParseError(HexParseError),
    NoteNotFound(NoteId),
    MmrError(MmrError),
    NoteInclusionProofError(NoteError),
    NoteTagAlreadyTracked(u64),
    ParsingError(String),
    QueryError(String),
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
            DataDeserializationError(err) => {
                write!(f, "error deserializing data from the store: {err}")
            },
            DatabaseError(err) => write!(f, "database-related non-query error: {err}"),
            HexParseError(err) => {
                write!(f, "error parsing hex: {err}")
            },
            NoteNotFound(note_id) => {
                write!(f, "note with note id {} not found", note_id.inner())
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

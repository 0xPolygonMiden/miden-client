use alloc::{
    boxed::Box,
    string::{String, ToString},
};
use core::error::Error;

use miden_objects::{NoteError, account::AccountId, note::NoteId, utils::DeserializationError};
use thiserror::Error;

// RPC ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum RpcError {
    #[error("rpc api response contained an update for a private account: {0}")]
    AccountUpdateForPrivateAccountReceived(AccountId),
    #[error("failed to connect to the api server: {0}")]
    ConnectionError(#[source] Box<dyn Error + Send + Sync + 'static>),
    #[error("failed to deserialize rpc data: {0}")]
    DeserializationError(String),
    #[error("rpc api response missing an expected field: {0}")]
    ExpectedDataMissing(String),
    #[error("rpc api response is invalid: {0}")]
    InvalidResponse(String),
    #[error("note with id {0} was not found")]
    NoteNotFound(NoteId),
    #[error("rpc request failed for {0}: {1}")]
    RequestError(String, String),
}

impl From<DeserializationError> for RpcError {
    fn from(err: DeserializationError) -> Self {
        Self::DeserializationError(err.to_string())
    }
}

impl From<NoteError> for RpcError {
    fn from(err: NoteError) -> Self {
        Self::DeserializationError(err.to_string())
    }
}

impl From<RpcConversionError> for RpcError {
    fn from(err: RpcConversionError) -> Self {
        Self::DeserializationError(err.to_string())
    }
}

// RPC CONVERSION ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum RpcConversionError {
    #[error("value is not in the range 0..modulus")]
    NotAValidFelt,
    #[error("invalid note type value")]
    NoteTypeError(#[from] NoteError),
    #[error("failed to convert rpc data: {0}")]
    InvalidField(String),
    #[error("field `{field_name}` expected to be present in protobuf representation of {entity}")]
    MissingFieldInProtobufRepresentation {
        entity: &'static str,
        field_name: &'static str,
    },
}

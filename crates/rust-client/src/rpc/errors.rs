use alloc::string::{String, ToString};
use core::fmt;

use miden_objects::{accounts::AccountId, utils::DeserializationError, NoteError};

// RPC ERROR
// ================================================================================================

#[derive(Debug)]
pub enum RpcError {
    AccountUpdateForPrivateAccountReceived(AccountId),
    ConnectionError(String),
    DeserializationError(String),
    ExpectedDataMissing(String),
    InvalidResponse(String),
    RequestError(String, String),
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::AccountUpdateForPrivateAccountReceived(account_id) => {
                write!(
                    f,
                    "RPC API response contained an update for a private account: {}",
                    account_id.to_hex()
                )
            },
            RpcError::ConnectionError(err) => {
                write!(f, "failed to connect to the API server: {err}")
            },
            RpcError::DeserializationError(err) => {
                write!(f, "failed to deserialize RPC data: {err}")
            },
            RpcError::ExpectedDataMissing(err) => {
                write!(f, "RPC API response missing an expected field: {err}")
            },
            RpcError::InvalidResponse(err) => {
                write!(f, "RPC API response is invalid: {err}")
            },
            RpcError::RequestError(endpoint, err) => {
                write!(f, "RPC request failed for {endpoint}: {err}")
            },
        }
    }
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

#[derive(Debug, Clone, PartialEq)]
pub enum RpcConversionError {
    NotAValidFelt,
    NoteTypeError(NoteError),
    MissingFieldInProtobufRepresentation {
        entity: &'static str,
        field_name: &'static str,
    },
}

impl core::fmt::Display for RpcConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcConversionError::NotAValidFelt => write!(f, "Value is not in the range 0..MODULUS"),
            RpcConversionError::NoteTypeError(err) => write!(f, "Invalid note type value: {}", err),
            RpcConversionError::MissingFieldInProtobufRepresentation { entity, field_name } => {
                write!(
                    f,
                    "Field `{}` required to be filled in protobuf representation of {}",
                    field_name, entity
                )
            },
        }
    }
}

impl Eq for RpcConversionError {}

impl From<NoteError> for RpcConversionError {
    fn from(error: NoteError) -> Self {
        RpcConversionError::NoteTypeError(error)
    }
}

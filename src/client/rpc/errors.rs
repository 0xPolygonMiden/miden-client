use core::fmt;
#[cfg(not(feature = "tonic"))]
use std::any::type_name;

#[cfg(feature = "tonic")]
use miden_node_proto::errors::ConversionError;
#[cfg(not(feature = "tonic"))]
use miden_objects::crypto::merkle::{SmtLeafError, SmtProofError};
use miden_objects::{accounts::AccountId, utils::DeserializationError, NoteError};
#[cfg(not(feature = "tonic"))]
use thiserror::Error;

// CONVERSION ERROR (temporary until
// https://github.com/0xPolygonMiden/miden-client/pull/378#discussion_r1639948388 gets addressed)
// ================================================================================================
//
#[cfg(not(feature = "tonic"))]
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ConversionError {
    #[error("Hex error: {0}")]
    HexError(#[from] hex::FromHexError),
    #[error("SMT leaf error: {0}")]
    SmtLeafError(#[from] SmtLeafError),
    #[error("SMT proof error: {0}")]
    SmtProofError(#[from] SmtProofError),
    #[error("Too much data, expected {expected}, got {got}")]
    TooMuchData { expected: usize, got: usize },
    #[error("Not enough data, expected {expected}, got {got}")]
    InsufficientData { expected: usize, got: usize },
    #[error("Value is not in the range 0..MODULUS")]
    NotAValidFelt,
    #[error("Invalid note type value: {0}")]
    NoteTypeError(#[from] NoteError),
    #[error("Field `{field_name}` required to be filled in protobuf representation of {entity}")]
    MissingFieldInProtobufRepresentation {
        entity: &'static str,
        field_name: &'static str,
    },
}

#[cfg(not(feature = "tonic"))]
impl Eq for ConversionError {}

// TODO: temporary until https://github.com/0xPolygonMiden/miden-client/pull/378#discussion_r1639948388 gets addressed.
#[cfg(not(feature = "tonic"))]
#[allow(dead_code)]
pub trait MissingFieldHelper {
    fn missing_field(field_name: &'static str) -> ConversionError;
}

#[cfg(not(feature = "tonic"))]
impl<T: prost::Message> MissingFieldHelper for T {
    fn missing_field(field_name: &'static str) -> ConversionError {
        ConversionError::MissingFieldInProtobufRepresentation {
            entity: type_name::<T>(),
            field_name,
        }
    }
}

// RPC ERROR
// ================================================================================================

#[derive(Debug)]
pub enum RpcError {
    ConnectionError(String),
    DeserializationError(String),
    ExpectedFieldMissing(String),
    AccountUpdateForPrivateAccountReceived(AccountId),
    RequestError(String, String),
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::ConnectionError(err) => {
                write!(f, "failed to connect to the API server: {err}")
            },
            RpcError::DeserializationError(err) => {
                write!(f, "failed to deserialize RPC data: {err}")
            },
            RpcError::ExpectedFieldMissing(err) => {
                write!(f, "rpc API response missing an expected field: {err}")
            },
            RpcError::AccountUpdateForPrivateAccountReceived(account_id) => {
                write!(
                    f,
                    "rpc API response contained an update for a private account: {}",
                    account_id.to_hex()
                )
            },
            RpcError::RequestError(endpoint, err) => {
                write!(f, "rpc request failed for {endpoint}: {err}")
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

// ERROR CONVERSIONS
// ================================================================================================

impl From<ConversionError> for RpcError {
    fn from(err: ConversionError) -> Self {
        Self::DeserializationError(err.to_string())
    }
}

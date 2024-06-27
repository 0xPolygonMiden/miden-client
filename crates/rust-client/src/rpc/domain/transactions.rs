use miden_objects::{crypto::hash::rpo::RpoDigest, transaction::TransactionId};

use crate::rpc::errors::RpcConversionError;
#[cfg(feature = "tonic")]
use crate::rpc::tonic_client::generated::{
    digest::Digest, transaction::TransactionId as ProtoTransactionId,
};
#[cfg(feature = "web-tonic")]
use crate::rpc::web_tonic_client::generated::{
    digest::Digest, transaction::TransactionId as ProtoTransactionId,
};

// INTO TRANSACTION ID
// ================================================================================================

impl TryFrom<Digest> for TransactionId {
    type Error = RpcConversionError;

    fn try_from(value: Digest) -> Result<Self, Self::Error> {
        let digest: RpoDigest = value.try_into()?;
        Ok(digest.into())
    }
}

impl TryFrom<ProtoTransactionId> for TransactionId {
    type Error = RpcConversionError;

    fn try_from(value: ProtoTransactionId) -> Result<Self, Self::Error> {
        value
            .id
            .ok_or(RpcConversionError::MissingFieldInProtobufRepresentation {
                entity: "TransactionId",
                field_name: "id",
            })?
            .try_into()
    }
}

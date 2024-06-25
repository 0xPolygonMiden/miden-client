use miden_objects::{crypto::hash::rpo::RpoDigest, transaction::TransactionId};

use crate::{
    errors::ConversionError,
    rpc::tonic_client::generated::{digest::Digest, transaction::TransactionId as TransactionIdPb},
};

// INTO TRANSACTION ID
// ================================================================================================

impl TryFrom<Digest> for TransactionId {
    type Error = ConversionError;

    fn try_from(value: Digest) -> Result<Self, Self::Error> {
        let digest: RpoDigest = value.try_into()?;
        Ok(digest.into())
    }
}

impl TryFrom<TransactionIdPb> for TransactionId {
    type Error = ConversionError;

    fn try_from(value: TransactionIdPb) -> Result<Self, Self::Error> {
        value
            .id
            .ok_or(ConversionError::MissingFieldInProtobufRepresentation {
                entity: "TransactionId",
                field_name: "id",
            })?
            .try_into()
    }
}

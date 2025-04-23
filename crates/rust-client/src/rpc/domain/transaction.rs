use miden_objects::{account::AccountId, crypto::hash::rpo::RpoDigest, transaction::TransactionId};

use crate::rpc::{
    errors::RpcConversionError,
    generated::{digest::Digest, transaction::TransactionId as ProtoTransactionId},
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

impl From<TransactionId> for ProtoTransactionId {
    fn from(value: TransactionId) -> Self {
        Self { id: Some(value.inner().into()) }
    }
}

// TRANSACTION UPDATE
// ================================================================================================

/// Represents a transaction that was included in the node at a certain block.
#[derive(PartialEq, Eq)]
pub struct TransactionUpdate {
    /// The transaction identifier.
    pub transaction_id: TransactionId,
    /// The number of the block in which the transaction was included.
    pub block_num: u32,
    /// The account that the transcation was executed against.
    pub account_id: AccountId,
}

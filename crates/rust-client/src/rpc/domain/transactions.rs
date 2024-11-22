use miden_objects::{
    accounts::AccountId, crypto::hash::rpo::RpoDigest, transaction::TransactionId,
};

#[cfg(feature = "tonic")]
use crate::rpc::tonic_client::generated::{
    digest::Digest, transaction::TransactionId as ProtoTransactionId,
};
#[cfg(feature = "web-tonic")]
use crate::rpc::web_tonic_client::generated::{
    digest::Digest, transaction::TransactionId as ProtoTransactionId,
};
use crate::rpc::RpcConversionError;

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

/// Represents a transaction that was included in the node at a certain block.
pub struct TransactionUpdate {
    /// The transaction Identifier
    pub transaction_id: TransactionId,
    /// The number of the block in which the transaction was included.
    pub block_num: u32,
    /// The account that the transcation was executed against.
    pub account_id: AccountId,
}

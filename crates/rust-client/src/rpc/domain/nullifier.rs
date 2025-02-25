use miden_objects::{crypto::hash::rpo::RpoDigest, note::Nullifier};

use super::MissingFieldHelper;
use crate::rpc::{self, errors::RpcConversionError, generated::digest::Digest};

// NULLIFIER UPDATE
// ================================================================================================

/// Represents a note that was consumed in the node at a certain block.
#[derive(Debug, Clone)]
pub struct NullifierUpdate {
    /// The nullifier of the consumed note.
    pub nullifier: Nullifier,
    /// The number of the block in which the note consumption was registered.
    pub block_num: u32,
}

// CONVERSIONS
// ================================================================================================

impl TryFrom<Digest> for Nullifier {
    type Error = RpcConversionError;

    fn try_from(value: Digest) -> Result<Self, Self::Error> {
        let digest: RpoDigest = value.try_into()?;
        Ok(digest.into())
    }
}

impl TryFrom<&rpc::generated::responses::NullifierUpdate> for NullifierUpdate {
    type Error = RpcConversionError;

    fn try_from(value: &rpc::generated::responses::NullifierUpdate) -> Result<Self, Self::Error> {
        Ok(Self {
            nullifier: value
                .nullifier
                .ok_or(rpc::generated::responses::NullifierUpdate::missing_field("nullifier"))?
                .try_into()?,
            block_num: value.block_num,
        })
    }
}

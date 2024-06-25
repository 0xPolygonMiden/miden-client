use miden_objects::{crypto::hash::rpo::RpoDigest, notes::Nullifier};

use crate::{errors::ConversionError, rpc::tonic_client::generated::digest::Digest};

// INTO NULLIFIER
// ================================================================================================

impl TryFrom<Digest> for Nullifier {
    type Error = ConversionError;

    fn try_from(value: Digest) -> Result<Self, Self::Error> {
        let digest: RpoDigest = value.try_into()?;
        Ok(digest.into())
    }
}
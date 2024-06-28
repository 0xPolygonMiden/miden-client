use miden_objects::{crypto::hash::rpo::RpoDigest, notes::Nullifier};

use crate::{errors::RpcConversionError, rpc::web_tonic_client::generated::digest::Digest};

// INTO NULLIFIER
// ================================================================================================

impl TryFrom<Digest> for Nullifier {
    type Error = RpcConversionError;

    fn try_from(value: Digest) -> Result<Self, Self::Error> {
        let digest: RpoDigest = value.try_into()?;
        Ok(digest.into())
    }
}

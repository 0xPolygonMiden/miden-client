use miden_objects::{
    crypto::merkle::{MerklePath, MmrDelta},
    Digest,
};

#[cfg(feature = "tonic")]
use crate::rpc::tonic_client::generated;
#[cfg(feature = "web-tonic")]
use crate::rpc::web_tonic_client::generated;
use crate::rpc::RpcConversionError;

// MERKLE PATH
// ================================================================================================

impl TryFrom<generated::merkle::MerklePath> for MerklePath {
    type Error = RpcConversionError;

    fn try_from(merkle_path: generated::merkle::MerklePath) -> Result<Self, Self::Error> {
        merkle_path.siblings.into_iter().map(Digest::try_from).collect()
    }
}

// MMR DELTA
// ================================================================================================

impl From<MmrDelta> for generated::mmr::MmrDelta {
    fn from(value: MmrDelta) -> Self {
        let data = value.data.into_iter().map(generated::digest::Digest::from).collect();
        generated::mmr::MmrDelta { forest: value.forest as u64, data }
    }
}

impl TryFrom<generated::mmr::MmrDelta> for MmrDelta {
    type Error = RpcConversionError;

    fn try_from(value: generated::mmr::MmrDelta) -> Result<Self, Self::Error> {
        let data: Result<Vec<_>, RpcConversionError> =
            value.data.into_iter().map(Digest::try_from).collect();

        Ok(MmrDelta {
            forest: value.forest as usize,
            data: data?,
        })
    }
}

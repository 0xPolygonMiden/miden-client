use miden_objects::block::BlockHeader;

use super::MissingFieldHelper;
use crate::rpc::{errors::RpcConversionError, generated::block};

// BLOCK HEADER
// ================================================================================================

impl From<&BlockHeader> for block::BlockHeader {
    fn from(header: &BlockHeader) -> Self {
        Self {
            version: header.version(),
            prev_block_commitment: Some(header.prev_block_commitment().into()),
            block_num: header.block_num().as_u32(),
            chain_commitment: Some(header.chain_commitment().into()),
            account_root: Some(header.account_root().into()),
            nullifier_root: Some(header.nullifier_root().into()),
            note_root: Some(header.note_root().into()),
            tx_commitment: Some(header.tx_commitment().into()),
            tx_kernel_commitment: Some(header.tx_kernel_commitment().into()),
            proof_commitment: Some(header.proof_commitment().into()),
            timestamp: header.timestamp(),
        }
    }
}

impl From<BlockHeader> for block::BlockHeader {
    fn from(header: BlockHeader) -> Self {
        (&header).into()
    }
}

impl TryFrom<&block::BlockHeader> for BlockHeader {
    type Error = RpcConversionError;

    fn try_from(value: &block::BlockHeader) -> Result<Self, Self::Error> {
        (*value).try_into()
    }
}

impl TryFrom<block::BlockHeader> for BlockHeader {
    type Error = RpcConversionError;

    fn try_from(value: block::BlockHeader) -> Result<Self, Self::Error> {
        Ok(BlockHeader::new(
            value.version,
            value
                .prev_block_commitment
                .ok_or(block::BlockHeader::missing_field(stringify!(prev_block_commitment)))?
                .try_into()?,
            value.block_num.into(),
            value
                .chain_commitment
                .ok_or(block::BlockHeader::missing_field(stringify!(chain_commitment)))?
                .try_into()?,
            value
                .account_root
                .ok_or(block::BlockHeader::missing_field(stringify!(account_root)))?
                .try_into()?,
            value
                .nullifier_root
                .ok_or(block::BlockHeader::missing_field(stringify!(nullifier_root)))?
                .try_into()?,
            value
                .note_root
                .ok_or(block::BlockHeader::missing_field(stringify!(note_root)))?
                .try_into()?,
            value
                .tx_commitment
                .ok_or(block::BlockHeader::missing_field(stringify!(tx_commitment)))?
                .try_into()?,
            value
                .tx_kernel_commitment
                .ok_or(block::BlockHeader::missing_field(stringify!(tx_kernel_commitment)))?
                .try_into()?,
            value
                .proof_commitment
                .ok_or(block::BlockHeader::missing_field(stringify!(proof_commitment)))?
                .try_into()?,
            value.timestamp,
        ))
    }
}

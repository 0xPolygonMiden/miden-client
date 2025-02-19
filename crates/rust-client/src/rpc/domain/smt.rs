use alloc::string::ToString;

use miden_objects::{
    crypto::merkle::{LeafIndex, SmtLeaf, SmtProof, SMT_DEPTH},
    Digest, Word,
};

use super::MissingFieldHelper;
use crate::rpc::{errors::RpcConversionError, generated};

// SMT LEAF ENTRY
// ================================================================================================

impl From<&(Digest, Word)> for generated::smt::SmtLeafEntry {
    fn from(value: &(Digest, Word)) -> Self {
        generated::smt::SmtLeafEntry {
            key: Some(value.0.into()),
            value: Some(Digest::new(value.1).into()),
        }
    }
}

impl TryFrom<&generated::smt::SmtLeafEntry> for (Digest, Word) {
    type Error = RpcConversionError;

    fn try_from(value: &generated::smt::SmtLeafEntry) -> Result<Self, Self::Error> {
        let key = match value.key {
            Some(key) => key.try_into()?,
            None => return Err(generated::smt::SmtLeafEntry::missing_field("key")),
        };

        let value: Digest = match value.value {
            Some(value) => value.try_into()?,
            None => return Err(generated::smt::SmtLeafEntry::missing_field("value")),
        };

        Ok((key, value.into()))
    }
}

// SMT LEAF
// ================================================================================================

impl From<SmtLeaf> for generated::smt::SmtLeaf {
    fn from(value: SmtLeaf) -> Self {
        (&value).into()
    }
}

impl From<&SmtLeaf> for generated::smt::SmtLeaf {
    fn from(value: &SmtLeaf) -> Self {
        match value {
            SmtLeaf::Empty(index) => generated::smt::SmtLeaf {
                leaf: Some(generated::smt::smt_leaf::Leaf::Empty(index.value())),
            },
            SmtLeaf::Single(entry) => generated::smt::SmtLeaf {
                leaf: Some(generated::smt::smt_leaf::Leaf::Single(entry.into())),
            },
            SmtLeaf::Multiple(entries) => generated::smt::SmtLeaf {
                leaf: Some(generated::smt::smt_leaf::Leaf::Multiple(
                    generated::smt::SmtLeafEntries {
                        entries: entries.iter().map(Into::into).collect(),
                    },
                )),
            },
        }
    }
}

impl TryFrom<&generated::smt::SmtLeaf> for SmtLeaf {
    type Error = RpcConversionError;

    fn try_from(value: &generated::smt::SmtLeaf) -> Result<Self, Self::Error> {
        match &value.leaf {
            Some(generated::smt::smt_leaf::Leaf::Empty(index)) => Ok(SmtLeaf::Empty(
                LeafIndex::<SMT_DEPTH>::new(*index)
                    .map_err(|err| RpcConversionError::InvalidField(err.to_string()))?,
            )),
            Some(generated::smt::smt_leaf::Leaf::Single(entry)) => {
                Ok(SmtLeaf::Single(entry.try_into()?))
            },
            Some(generated::smt::smt_leaf::Leaf::Multiple(entries)) => {
                let entries =
                    entries.entries.iter().map(TryInto::try_into).collect::<Result<_, _>>()?;
                Ok(SmtLeaf::Multiple(entries))
            },
            None => Err(generated::smt::SmtLeaf::missing_field("leaf")),
        }
    }
}

// SMT PROOF
// ================================================================================================

impl From<SmtProof> for generated::smt::SmtOpening {
    fn from(value: SmtProof) -> Self {
        (&value).into()
    }
}

impl From<&SmtProof> for generated::smt::SmtOpening {
    fn from(value: &SmtProof) -> Self {
        generated::smt::SmtOpening {
            leaf: Some(value.leaf().into()),
            path: Some(value.path().into()),
        }
    }
}

impl TryFrom<&generated::smt::SmtOpening> for SmtProof {
    type Error = RpcConversionError;

    fn try_from(value: &generated::smt::SmtOpening) -> Result<Self, Self::Error> {
        let leaf = match &value.leaf {
            Some(leaf) => leaf.try_into()?,
            None => return Err(generated::smt::SmtOpening::missing_field("leaf")),
        };

        let path = match &value.path {
            Some(path) => path.try_into()?,
            None => return Err(generated::smt::SmtOpening::missing_field("path")),
        };

        SmtProof::new(path, leaf).map_err(|err| RpcConversionError::InvalidField(err.to_string()))
    }
}

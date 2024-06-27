use std::fmt::{Debug, Display, Formatter};

use hex::ToHex;
use miden_objects::{notes::NoteId, Digest, Felt, StarkField};

use crate::rpc::errors::RpcConversionError;
#[cfg(feature = "tonic")]
use crate::rpc::tonic_client::generated::digest;
#[cfg(feature = "web-tonic")]
use crate::rpc::web_tonic_client::generated::digest;

// CONSTANTS
// ================================================================================================

pub const DIGEST_DATA_SIZE: usize = 32;

// FORMATTING
// ================================================================================================

impl Display for digest::Digest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.encode_hex::<String>())
    }
}

impl Debug for digest::Digest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl ToHex for &digest::Digest {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        (*self).encode_hex()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        (*self).encode_hex_upper()
    }
}

impl ToHex for digest::Digest {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        let mut data: Vec<char> = Vec::with_capacity(DIGEST_DATA_SIZE);
        data.extend(format!("{:016x}", self.d0).chars());
        data.extend(format!("{:016x}", self.d1).chars());
        data.extend(format!("{:016x}", self.d2).chars());
        data.extend(format!("{:016x}", self.d3).chars());
        data.into_iter().collect()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        let mut data: Vec<char> = Vec::with_capacity(DIGEST_DATA_SIZE);
        data.extend(format!("{:016X}", self.d0).chars());
        data.extend(format!("{:016X}", self.d1).chars());
        data.extend(format!("{:016X}", self.d2).chars());
        data.extend(format!("{:016X}", self.d3).chars());
        data.into_iter().collect()
    }
}

// INTO
// ================================================================================================

impl From<Digest> for digest::Digest {
    fn from(value: Digest) -> Self {
        Self {
            d0: value[0].as_int(),
            d1: value[1].as_int(),
            d2: value[2].as_int(),
            d3: value[3].as_int(),
        }
    }
}

impl From<&Digest> for digest::Digest {
    fn from(value: &Digest) -> Self {
        (*value).into()
    }
}

impl From<&NoteId> for digest::Digest {
    fn from(value: &NoteId) -> Self {
        (*value).inner().into()
    }
}

impl From<NoteId> for digest::Digest {
    fn from(value: NoteId) -> Self {
        value.inner().into()
    }
}

// FROM DIGEST
// ================================================================================================

impl TryFrom<digest::Digest> for [Felt; 4] {
    type Error = RpcConversionError;

    fn try_from(value: digest::Digest) -> Result<Self, Self::Error> {
        if ![value.d0, value.d1, value.d2, value.d3]
            .iter()
            .all(|v| *v < <Felt as StarkField>::MODULUS)
        {
            Err(RpcConversionError::NotAValidFelt)
        } else {
            Ok([
                Felt::new(value.d0),
                Felt::new(value.d1),
                Felt::new(value.d2),
                Felt::new(value.d3),
            ])
        }
    }
}

impl TryFrom<digest::Digest> for Digest {
    type Error = RpcConversionError;

    fn try_from(value: digest::Digest) -> Result<Self, Self::Error> {
        Ok(Self::new(value.try_into()?))
    }
}

impl TryFrom<&digest::Digest> for [Felt; 4] {
    type Error = RpcConversionError;

    fn try_from(value: &digest::Digest) -> Result<Self, Self::Error> {
        value.clone().try_into()
    }
}

impl TryFrom<&digest::Digest> for Digest {
    type Error = RpcConversionError;

    fn try_from(value: &digest::Digest) -> Result<Self, Self::Error> {
        value.clone().try_into()
    }
}

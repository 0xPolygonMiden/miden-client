use std::fmt::{Debug, Display, Formatter};

use miden_objects::accounts::AccountId;

use crate::rpc::errors::RpcConversionError;
#[cfg(feature = "tonic")]
use crate::rpc::tonic_client::generated::account::AccountId as ProtoAccountId;
#[cfg(feature = "web-tonic")]
use crate::rpc::web_tonic_client::generated::account::AccountId as ProtoAccountId;

// ACCOUNT ID
// ================================================================================================

impl Display for ProtoAccountId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("0x{:x}", self.id))
    }
}

impl Debug for ProtoAccountId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

// INTO PROTO ACCOUNT ID
// ------------------------------------------------------------------------------------------------

impl From<AccountId> for ProtoAccountId {
    fn from(account_id: AccountId) -> Self {
        Self { id: account_id.into() }
    }
}

// FROM PROTO ACCOUNT ID
// ------------------------------------------------------------------------------------------------

impl TryFrom<ProtoAccountId> for AccountId {
    type Error = RpcConversionError;

    fn try_from(account_id: ProtoAccountId) -> Result<Self, Self::Error> {
        account_id.id.try_into().map_err(|_| RpcConversionError::NotAValidFelt)
    }
}

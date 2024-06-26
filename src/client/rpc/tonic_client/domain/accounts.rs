use std::fmt::{Debug, Display, Formatter};

use miden_objects::accounts::AccountId;

use crate::{
    errors::RpcConversionError, rpc::tonic_client::generated::account::AccountId as AccountIdPb,
};

// ACCOUNT ID
// ================================================================================================

impl Display for AccountIdPb {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("0x{:x}", self.id))
    }
}

impl Debug for AccountIdPb {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

// INTO PROTO ACCOUNT ID
// ------------------------------------------------------------------------------------------------

impl From<AccountId> for AccountIdPb {
    fn from(account_id: AccountId) -> Self {
        Self { id: account_id.into() }
    }
}

// FROM PROTO ACCOUNT ID
// ------------------------------------------------------------------------------------------------

impl TryFrom<AccountIdPb> for AccountId {
    type Error = RpcConversionError;

    fn try_from(account_id: AccountIdPb) -> Result<Self, Self::Error> {
        account_id.id.try_into().map_err(|_| RpcConversionError::NotAValidFelt)
    }
}
